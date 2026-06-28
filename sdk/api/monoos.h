/**
 * monoos.h – MonoOS Public SDK – Master include
 *
 * Include this single header to access all MonoOS SDK APIs.
 * Concrete sub-headers are pulled in below; they may also be included
 * individually when only a subset of the API is needed.
 *
 * Language compatibility: C11 and C++17.
 *
 * Example (C):
 *   #include <monoos/monoos.h>
 *   int main(void) {
 *       MonoOS_Context *ctx = monoos_context_create("com.example.app", 1);
 *       monoos_context_destroy(ctx);
 *       return 0;
 *   }
 */

#pragma once
#ifndef MONOOS_SDK_H
#define MONOOS_SDK_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/* ── Version ─────────────────────────────────────────────────────────────── */
#define MONOOS_SDK_VERSION_MAJOR  1
#define MONOOS_SDK_VERSION_MINOR  0
#define MONOOS_SDK_VERSION_PATCH  0
#define MONOOS_SDK_VERSION_STRING "1.0.0"

/** Packed version integer for compile-time comparisons. */
#define MONOOS_SDK_VERSION \
    ((MONOOS_SDK_VERSION_MAJOR << 16) | \
     (MONOOS_SDK_VERSION_MINOR <<  8) | \
      MONOOS_SDK_VERSION_PATCH)

/** Minimum SDK version an app must declare to use this header. */
#define MONOOS_MIN_SDK  1

/* ── Result codes ────────────────────────────────────────────────────────── */
typedef int32_t MonoOS_Result;

#define MONOOS_OK                      0
#define MONOOS_ERROR                  -1
#define MONOOS_ERROR_INVALID_ARG      -2
#define MONOOS_ERROR_PERMISSION_DENIED -3
#define MONOOS_ERROR_NOT_FOUND        -4
#define MONOOS_ERROR_ALREADY_EXISTS   -5
#define MONOOS_ERROR_NO_MEMORY        -6
#define MONOOS_ERROR_IO               -7
#define MONOOS_ERROR_TIMEOUT          -8
#define MONOOS_ERROR_NOT_SUPPORTED    -9
#define MONOOS_ERROR_NOT_INITIALISED  -10

/** Return the human-readable name of a result code. */
const char *monoos_result_str(MonoOS_Result r);

/* ── Application Context ─────────────────────────────────────────────────── */
typedef struct MonoOS_Context MonoOS_Context;

/**
 * Create an application context.
 *
 * @param package_name  Reverse-DNS package identifier (e.g. "com.example.app").
 * @param version_code  Integer version of the application.
 * @return              Heap-allocated context, or NULL on failure.
 */
MonoOS_Context *monoos_context_create(const char *package_name,
                                     uint32_t    version_code);

/** Destroy a previously created context and free all associated resources. */
void monoos_context_destroy(MonoOS_Context *ctx);

/** Return the package name stored in this context. */
const char *monoos_context_package_name(const MonoOS_Context *ctx);

/* ── Sub-headers ─────────────────────────────────────────────────────────── */
#include "monoos_permissions.h"
#include "monoos_storage.h"
#include "monoos_notifications.h"
#include "monoos_audio.h"
#include "monoos_media.h"
#include "monoos_network.h"
#include "monoos_ui.h"

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* MONOOS_SDK_H */
