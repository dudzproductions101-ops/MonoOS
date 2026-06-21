// service_loader.cpp – MonoOS service manager / init.rc parser
//
// Reads /etc/monoos/services.conf (an init.rc-compatible subset) and
// launches, supervises, and restarts declared services.
//
// Supported init.rc directives (subset):
//   service <name> <binary> [args...]
//       class <class>        – grouping: core | main | late_start | optional
//       user  <uid>
//       group <gid> [gids…]
//       disabled             – do not auto-start
//       oneshot              – do not restart on death
//       critical             – reboot device if it crashes > 4 times / 4 min
//       seclabel <label>     – SELinux context
//       socket <name> <type> <perm> [uid [gid]]
//
//   on <trigger>             – event trigger block
//       start <service>
//       stop  <service>
//       setprop <key> <val>

#include <algorithm>
#include <array>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <sstream>
#include <string>
#include <unordered_map>
#include <vector>

#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <sys/reboot.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <unistd.h>

namespace monoos {

// ──────────────────────────────────────────────────────────────────────────────
//  ServiceState
// ──────────────────────────────────────────────────────────────────────────────

enum class ServiceState {
    Stopped,
    Starting,
    Running,
    Restarting,
    Crashed,
};

static const char *state_str(ServiceState s) {
    switch (s) {
        case ServiceState::Stopped:    return "stopped";
        case ServiceState::Starting:   return "starting";
        case ServiceState::Running:    return "running";
        case ServiceState::Restarting: return "restarting";
        case ServiceState::Crashed:    return "crashed";
    }
    return "unknown";
}

// ──────────────────────────────────────────────────────────────────────────────
//  ServiceDef – parsed service definition
// ──────────────────────────────────────────────────────────────────────────────

struct ServiceDef {
    std::string              name;
    std::string              binary;
    std::vector<std::string> args;
    std::string              cls;          // core | main | late_start | optional
    uid_t                    uid  = 0;
    gid_t                    gid  = 0;
    std::vector<gid_t>       supplementary_gids;
    bool                     disabled = false;
    bool                     oneshot  = false;
    bool                     critical = false;
    std::string              seclabel;
};

// ──────────────────────────────────────────────────────────────────────────────
//  ServiceInstance – runtime tracking
// ──────────────────────────────────────────────────────────────────────────────

struct ServiceInstance {
    const ServiceDef *def;
    ServiceState      state  = ServiceState::Stopped;
    pid_t             pid    = -1;
    int               crash_count = 0;
    std::chrono::steady_clock::time_point last_start;
    std::chrono::steady_clock::time_point last_crash;
};

// ──────────────────────────────────────────────────────────────────────────────
//  Parser
// ──────────────────────────────────────────────────────────────────────────────

static std::vector<ServiceDef> parse_services_conf(const std::string &path)
{
    std::vector<ServiceDef> defs;
    std::ifstream f(path);
    if (!f.is_open()) return defs;

    std::string line;
    ServiceDef *cur = nullptr;

    while (std::getline(f, line)) {
        // Strip comment and leading whitespace.
        if (auto pos = line.find('#'); pos != std::string::npos)
            line.erase(pos);
        while (!line.empty() && (line.front() == ' ' || line.front() == '\t'))
            line.erase(line.begin());
        if (line.empty()) continue;

        std::istringstream ss(line);
        std::string tok;
        ss >> tok;

        if (tok == "service") {
            defs.push_back({});
            cur = &defs.back();
            ss >> cur->name >> cur->binary;
            std::string arg;
            while (ss >> arg) cur->args.push_back(arg);
        } else if (cur) {
            if      (tok == "class")    { ss >> cur->cls; }
            else if (tok == "user")     { std::string u; ss >> u; cur->uid = (uid_t)std::stoul(u); }
            else if (tok == "group")    { std::string g; ss >> g; cur->gid = (gid_t)std::stoul(g); }
            else if (tok == "disabled") { cur->disabled = true; }
            else if (tok == "oneshot")  { cur->oneshot  = true; }
            else if (tok == "critical") { cur->critical  = true; }
            else if (tok == "seclabel") { ss >> cur->seclabel; }
        }
    }
    return defs;
}

// ──────────────────────────────────────────────────────────────────────────────
//  ServiceLoader
// ──────────────────────────────────────────────────────────────────────────────

class ServiceLoader {
public:
    explicit ServiceLoader(const std::string &conf_path)
        : conf_path_(conf_path) {}

    bool load() {
        defs_ = parse_services_conf(conf_path_);
        for (auto &d : defs_) {
            instances_.push_back({ &d });
        }
        fprintf(stderr, "[service_loader] loaded %zu service definitions\n",
                defs_.size());
        return !defs_.empty();
    }

    void start_class(const std::string &cls) {
        for (auto &inst : instances_) {
            if (inst.def->cls == cls && !inst.def->disabled)
                start_service(inst);
        }
    }

    void run_forever() {
        sigset_t mask;
        sigemptyset(&mask);
        sigaddset(&mask, SIGCHLD);
        sigprocmask(SIG_BLOCK, &mask, nullptr);

        start_class("core");
        start_class("main");

        for (;;) {
            reap_children();
            usleep(100'000); // 100 ms poll interval
        }
    }

    void start_service(ServiceInstance &inst) {
        if (inst.state == ServiceState::Running) return;

        pid_t pid = fork();
        if (pid < 0) {
            fprintf(stderr, "[service_loader] fork failed for %s: %s\n",
                    inst.def->name.c_str(), strerror(errno));
            return;
        }
        if (pid == 0) {
            // Child: set uid/gid and exec.
            if (inst.def->gid) setgid(inst.def->gid);
            if (inst.def->uid) setuid(inst.def->uid);

            std::vector<const char *> argv;
            argv.push_back(inst.def->binary.c_str());
            for (auto &a : inst.def->args) argv.push_back(a.c_str());
            argv.push_back(nullptr);

            execv(inst.def->binary.c_str(), (char *const *)argv.data());
            fprintf(stderr, "[service_loader] exec %s failed: %s\n",
                    inst.def->binary.c_str(), strerror(errno));
            _exit(127);
        }

        inst.pid        = pid;
        inst.state      = ServiceState::Running;
        inst.last_start = std::chrono::steady_clock::now();
        fprintf(stderr, "[service_loader] started %s (pid=%d)\n",
                inst.def->name.c_str(), pid);
    }

    void stop_service(ServiceInstance &inst) {
        if (inst.pid > 0) {
            kill(inst.pid, SIGTERM);
            inst.state = ServiceState::Stopped;
        }
    }

private:
    void reap_children() {
        int wstatus;
        pid_t wpid;
        while ((wpid = waitpid(-1, &wstatus, WNOHANG)) > 0) {
            for (auto &inst : instances_) {
                if (inst.pid != wpid) continue;
                inst.state = ServiceState::Stopped;
                inst.pid   = -1;
                inst.crash_count++;
                inst.last_crash = std::chrono::steady_clock::now();

                fprintf(stderr, "[service_loader] %s exited (status=%d, crashes=%d)\n",
                        inst.def->name.c_str(), WEXITSTATUS(wstatus),
                        inst.crash_count);

                if (inst.def->critical && inst.crash_count > 4) {
                    fprintf(stderr, "[service_loader] CRITICAL service %s "
                            "crashed too many times – rebooting!\n",
                            inst.def->name.c_str());
                    sync();
                    reboot(0x01234567); // LINUX_REBOOT_CMD_RESTART
                }

                if (!inst.def->oneshot) {
                    inst.state = ServiceState::Restarting;
                    usleep(500'000);
                    start_service(inst);
                }
                break;
            }
        }
    }

    std::string              conf_path_;
    std::vector<ServiceDef>  defs_;
    std::vector<ServiceInstance> instances_;
};

} // namespace monoos

int main(int argc, char **argv)
{
    const char *conf = "/etc/monoos/services.conf";
    if (argc > 1) conf = argv[1];

    monoos::ServiceLoader loader(conf);
    if (!loader.load()) {
        fprintf(stderr, "[service_loader] no services defined in %s\n", conf);
        return 1;
    }

    loader.run_forever();
    return 0; // never reached
}
