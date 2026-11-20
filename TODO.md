# TODO

Open work only.

- [ ] Quagmire's records and timeline onto `ouro-play` (it keeps its own `records.rs` and
      `timeline.rs`, which came first; S.C.Out and Harry use `ouro-play`)
- [ ] Gamepad mapping onto `Key` shared by the ports (now each port maps it)
- [ ] The pack library's screen (rule 8: approved datasets from a server,
      filtered by game; its config file's fields) once the server exists
- [ ] The system's file dialog as a second way in the browser (SDL3
      `SDL_ShowOpenFileDialog`; leaves full screen while open)
- [ ] A hook for a game's own conversion when adding a file: an original's fan
      file (Harry's custom worlds `Hwld`, data files `Hrdt`) goes through the
      game's extract code, with progress, and is kept as an Ouro dataset
      (decided 2026-11-19: only Harry needs it; Quagmire, Sammy and S.C.Out
      took no add-on files)
