#ifndef GUI_H
#define GUI_H

#include <QString>

int start(int argc, char* argv[]);

extern "C++" {
    void callback_run_lookup_query(QString query_text = "");
    void callback_run_summary_query(QString window_id, QString query_text = "");
    void callback_run_sutta_menu_action(QString window_id, QString action, QString query_text = "");
}

void open_sutta_search_window(QString query_text = "");

#endif
