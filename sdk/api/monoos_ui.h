/**
 * monoos_ui.h – MonoOS Application UI Loader API
 *
 * Apps built against the QML application template load their root UI
 * document through this API once their `MonoOS_Context` has been created.
 */

#pragma once
#ifndef MONOOS_UI_H
#define MONOOS_UI_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Load and display a QML document as the app's root UI surface.
 *
 * @param qml_path  Path relative to the package root (e.g. "res/qml/Main.qml").
 * @return MONOOS_OK on success, MONOOS_ERROR_NOT_FOUND if the file is missing,
 *         or MONOOS_ERROR if QML parsing/instantiation failed.
 */
int monoos_ui_load_qml(const char *qml_path);

/** Reload the currently displayed QML document (useful during development). */
int monoos_ui_reload(void);

/** Set whether the system status bar is visible over this app's window. */
void monoos_ui_set_status_bar_visible(bool visible);

/** Request the app's window be drawn edge-to-edge, behind system bars. */
void monoos_ui_set_edge_to_edge(bool enabled);

#ifdef __cplusplus
}
#endif
#endif /* MONOOS_UI_H */
