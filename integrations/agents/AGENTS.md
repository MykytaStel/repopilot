## Check your change with RepoPilot before you finish

This repository reviews every change with [RepoPilot](https://github.com/MykytaStel/repopilot).

1. When you start a task, run `repopilot snapshot` once.
2. Before you report the task as done, run `repopilot review --since-snapshot`.
3. If the output lists **Checks this change weakened**, restore each listed
   test, assertion, helper check, suppression, or CI gate. If a change to a
   test is intended, say why in your final message.
4. If the output flags a broken local caller (`removed export is still imported`
   or a Rust public API contract) or untrusted input reaching a SQL, shell, or
   filesystem path sink, fix the caller or add the missing guard before finishing.
5. To inspect one signal's provenance, impact path, and verification steps, run
   `repopilot review --since-snapshot --explain-signal <signal_id>`.
6. Never skip, delete, loosen, or short-circuit a test with `exit(0)`, and
   never add a lint, type, or coverage suppression only to make a check pass.
