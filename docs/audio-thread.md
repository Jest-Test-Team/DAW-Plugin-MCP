# Audio thread safety

`daw-plugin` `process()` / realtime callback may only:

- write into preallocated `AudioRing` / `MidiRing`
- read atomics

It must not:

- allocate
- take a mutex
- touch the filesystem
- open sockets
- call Julia
- call an LLM

IPC (`daw_plugin::ipc`) and Julia (`daw-analysis`) run on the daemon / message thread only.
