#ifndef GUI_H
#define GUI_H

#include <QString>

int start(int argc, char* argv[]);

extern "C++" {
    void callback_run_lookup_query(QString query_text = "");
}

void open_sutta_search_window(QString query_text = "");

#endif
