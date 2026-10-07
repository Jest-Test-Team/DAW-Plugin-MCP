*** Settings ***
Library    ${CURDIR}/libraries/McpClient.py
Suite Teardown    Stop Server

*** Test Cases ***
Http Health Binds Loopback Only
    ${body}=    Start Http Server    reaper    8765
    Should Contain    ${body}    127.0.0.1
    ${status}    ${health}=    Http Get    /health
    Should Be Equal As Integers    ${status}    200
    Should Contain    ${health}    "bind":"127.0.0.1"
    Should Not Contain    ${health}    0.0.0.0
    ${leaks}=    Response Leaks Home    ${health}
    Should Not Be True    ${leaks}

Logic Parameter Write Is Error And Does Not Leak Home
    Start Server    logic
    ${result}=    Call Tool    daw_set_parameter    {"plugin_id":"fx-1","name":"gain","value":0.5}
    Should Be True    ${result}[isError]
    ${text}=    Set Variable    ${result}[content][0][text]
    Should Contain    ${text}    unsupported
    ${leaks}=    Response Leaks Home    ${text}
    Should Not Be True    ${leaks}
