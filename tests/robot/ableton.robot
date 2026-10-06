*** Settings ***
Library    ${CURDIR}/libraries/McpClient.py
Suite Setup    Start Server    ableton
Suite Teardown    Stop Server

*** Test Cases ***
Ableton Is A Native Host In This Wave
    ${caps}=    Call Tool    daw_get_capabilities
    Should Be Equal    ${caps}[structuredContent][host]    ableton
    ${page}=    Call Tool    daw_list_tracks    {"limit":10,"offset":0}
    Should Be True    ${page}[structuredContent][total_count] >= 1
    ${names}=    Evaluate    [t['name'] for t in $page['structuredContent']['items']]
    Should Contain    ${names}    Kick
