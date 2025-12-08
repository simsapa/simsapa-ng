#ifndef GUI_H
#define GUI_H

#include <QString>

int start(int argc, char* argv[]);

extern "C++" {
    void callback_run_lookup_query(QString query_text = "");
    void callback_run_summary_query(QString window_id, QString query_text = "");
    void callback_run_sutta_menu_action(QString window_id, QString action, QString query_text = "");
    void callback_open_sutta_search_window(QString show_result_data_json = "");
    void callback_open_sutta_languages_window();
    void callback_open_library_window();
    void callback_show_chapter_in_sutta_window(QString result_data_json = "");
}

void open_sutta_search_window(QString query_text = "");

#endif
