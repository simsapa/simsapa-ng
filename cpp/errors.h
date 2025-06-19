#ifndef ERRORS_H_
#define ERRORS_H_

#include <stdexcept>
#include <string>

class FatalError : public std::runtime_error {
public:
    explicit FatalError(const std::string& message);

    explicit FatalError(const std::string& message, int error_code);

    int get_error_code() const;

private:
    int error_code_;
};

class NormalExit : public std::runtime_error {
public:
    explicit NormalExit(const std::string& message);

    explicit NormalExit(const std::string& message, int error_code);

    int get_error_code() const;

private:
    int error_code_;
};

#endif // ERRORS_H_
