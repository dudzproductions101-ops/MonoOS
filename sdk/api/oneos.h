/**
 * oneos.h – OneOS Public SDK – Master include
 *
 * Include this single header to access all OneOS SDK APIs.
 * Concrete sub-headers are pulled in below; they may also be included
 * individually when only a subset of the API is needed.
 *
 * Language compatibility: C11 and C++17.
 *
 * Example (C):
 *   #include <oneos/oneos.h>
 *   int main(void) {
 *       OneOS_Context *ctx = oneos_context_create("com.example.app", 1);
 *       oneos_context_destroy(ctx);
 *       return 0;
 *   }
 */

#pragma once
#ifndef ONEOS_SDK_H
#define ONEOS_SDK_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/* ── Version ─────────────────────────────────────────────────────────────── */
#define ONEOS_SDK_VERSION_MAJOR  1
#define ONEOS_SDK_VERSION_MINOR  0
#define ONEOS_SDK_VERSION_PATCH  0
#define ONEOS_SDK_VERSION_STRING "1.0.0"

/** Packed version integer for compile-time comparisons. */
#define ONEOS_SDK_VERSION \
    ((ONEOS_SDK_VERSION_MAJOR << 16) | \
     (ONEOS_SDK_VERSION_MINOR <<  8) | \
      ONEOS_SDK_VERSION_PATCH)

/** Minimum SDK version an app must declare to use this header. */
#define ONEOS_MIN_SDK  1

/* ── Result codes ────────────────────────────────────────────────────────── */
typedef int32_t OneoS_Result;

#define ONEOS_OK                      0
#define ONEOS_ERROR                  -1
#define ONEOS_ERROR_INVALID_ARG      -2
#define ONEOS_ERROR_PERMISSION_DENIED -3
#define ONEOS_ERROR_NOT_FOUND        -4
#define ONEOS_ERROR_ALREADY_EXISTS   -5
#define ONEOS_ERROR_NO_MEMORY        -6
#define ONEOS_ERROR_IO               -7
#define ONEOS_ERROR_TIMEOUT          -8
#define ONEOS_ERROR_NOT_SUPPORTED    -9
#define ONEOS_ERROR_NOT_INITIALISED  -10

/** Return the human-readable name of a result code. */
const char *oneos_result_str(OneoS_Result r);

/* ── Application Context ─────────────────────────────────────────────────── */
typedef struct OneOS_Context OneOS_Context;

/**
 * Create an application context.
 *
 * @param package_name  Reverse-DNS package identifier (e.g. "com.example.app").
 * @param version_code  Integer version of the application.
 * @return              Heap-allocated context, or NULL on failure.
 */
OneOS_Context *oneos_context_create(const char *package_name,
                                     uint32_t    version_code);

/** Destroy a previously created context and free all associated resources. */
void oneos_context_destroy(OneOS_Context *ctx);

/** Return the package name stored in this context. */
const char *oneos_context_package_name(const OneOS_Context *ctx);

/* ── Sub-headers ─────────────────────────────────────────────────────────── */
#include "oneos_permissions.h"
#include "oneos_storage.h"
#include "oneos_notifications.h"
#include "oneos_audio.h"
#include "oneos_media.h"
#include "oneos_network.h"

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ONEOS_SDK_H */
