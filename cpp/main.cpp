#include <iostream>

#include "errors.h"
#include "gui.h"

int main(int argc, char* argv[]) {
  try {
    int app_ret = start(argc, argv);
    return app_ret;

  } catch (const NormalExit& e) {
    std::cerr << "Normal exit: " << e.what() << std::endl;
    std::cerr << "Error code: " << e.get_error_code() << std::endl;
    return e.get_error_code();

  } catch (const FatalError& e) {
    std::cerr << "FATAL ERROR: " << e.what() << std::endl;
    std::cerr << "Error code: " << e.get_error_code() << std::endl;
    return e.get_error_code();

  } catch (const std::exception &e) {
    std::cerr << "Unexpected error: " << e.what() << std::endl;
    return 3;

  } catch (...) {
    std::cerr << "Unknown error occurred!" << std::endl;
    return 4;
  }
}
