*** Settings ***
Library    ${CURDIR}/libraries/McpClient.py
Suite Setup    Start Server    reaper
Suite Teardown    Stop Server

*** Test Cases ***
Mute Then Rollback Restores Track
    ${plan}=    Call Tool    daw_create_plan    {"confirmed":true,"actions":[{"id":"1","tool":"daw_set_track_mute","risk":"reversible","params":{"track_id":"trk-1","mute":true}}]}
    Should Not Be True    ${plan}[isError]
    ${id}=    Set Variable    ${plan}[structuredContent][id]
    Call Tool    daw_commit_plan    {"confirmed":true}
    ${muted}=    Call Tool    daw_get_track    {"track_id":"trk-1"}
    Should Be True    ${muted}[structuredContent][track][mute]
    Call Tool    daw_rollback    {"plan_id":"${id}"}
    ${restored}=    Call Tool    daw_get_track    {"track_id":"trk-1"}
    Should Not Be True    ${restored}[structuredContent][track][mute]
