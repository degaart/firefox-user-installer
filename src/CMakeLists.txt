set(PROGRAM_NAME firefox)

find_package(wxWidgets REQUIRED core base)
include(${wxWidgets_USE_FILE})

find_package(CURL REQUIRED COMPONENTS HTTP HTTPS)

add_executable(${PROGRAM_NAME}
    app.cpp
    gui.cpp
    installer.cpp
    util.cpp
)
target_compile_features(${PROGRAM_NAME} PRIVATE cxx_std_17)
target_link_libraries(${PROGRAM_NAME} PRIVATE
    ${wxWidgets_LIBRARIES}
    CURL::libcurl
)
