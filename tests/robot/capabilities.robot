*** Settings ***
Library    ${CURDIR}/libraries/McpClient.py
Suite Teardown    Stop Server

*** Test Cases ***
Reaper Lists Tracks As Supported
    Start Server    reaper
    ${caps}=    Call Tool    daw_get_capabilities
    Should Not Be True    ${caps}[isError]
    ${host}=    Set Variable    ${caps}[structuredContent][host]
    Should Be Equal    ${host}    reaper
    ${summary}=    Call Tool    daw_get_session_summary
    Should Be Equal As Integers    ${summary}[structuredContent][track_count]    2

Logic Marks Parameter Writes Unavailable
    Start Server    logic
    ${result}=    Call Tool    daw_set_parameter    {"plugin_id":"fx-1","name":"gain","value":0.5}
    Should Be True    ${result}[isError]
    ${text}=    Set Variable    ${result}[content][0][text]
    Should Contain    ${text}    unsupported
