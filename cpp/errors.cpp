#include "errors.h"

FatalError::FatalError(const std::string& message)
    : std::runtime_error(message), error_code_(1) {
}

FatalError::FatalError(const std::string& message, int error_code)
    : std::runtime_error(message), error_code_(error_code) {
}

int FatalError::get_error_code() const {
    return error_code_;
}

NormalExit::NormalExit(const std::string& message)
    : std::runtime_error(message), error_code_(1) {
}

NormalExit::NormalExit(const std::string& message, int error_code)
    : std::runtime_error(message), error_code_(error_code) {
}

int NormalExit::get_error_code() const {
    return error_code_;
}
