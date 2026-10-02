## Check your change with RepoPilot before you finish

This repository reviews every change with [RepoPilot](https://github.com/MykytaStel/repopilot).

1. When you start a task, run `repopilot snapshot` once.
2. Before you report the task as done, run `repopilot review --since-snapshot`.
3. If the output lists **Checks this change weakened**, restore each listed
   test, assertion, suppression, or CI gate. If a change to a test is
   intended, say why in your final message.
4. Never skip, delete, or loosen a test, and never add a lint, type, or
   coverage suppression, only to make a check pass.
