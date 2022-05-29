# Contributing

There are some guidelines you should follow when contributing to this
project:

1. Every commit should represent a single logical change. Squash or
separate one commit into several if necessary to achieve this.

2. Every commit should compile without commits after it. That is, if you
send 5 commits, combinations of `1st`, `1st + 2nd`, `1st + 2nd + 3th`,
`1st + 2nd + 3th + 4th` and `1st + 2nd + 3th + 4th + 5th` commits must
compile.

3. Every commit should have a proper name that describes what this commit
changes and why this is necessary. Add detailed text as a second
paragraph, unless the change is so trivial it requires no detailed
explanation. See [commit guidelines](
https://www.git-scm.com/book/en/v2/Distributed-Git-Contributing-to-a-Project
) for details.

4. Before sending patches or opening a pull request, rebase your branch
on `master` branch. Use either `git fetch --all && git rebase
origin/master master` or `git pull --rebase` when possible instead of
`git pull` 's default behavior.

5. If changes were requested, do not make new commits to implement those
changes, unless a new logical change is necessary. Instead, amend commits
and use other methods of changing history if necessary. See [git-rebase
.io](https://git-rebase.io) for guidelines. If you already opened a pull
request, feel free to force push your changes into your feature branch.
If you already sent a patch instead, send a new patch as a "v2" patch.
Consult [git-send-email.io](https://git-send-email.io) for details on how
to do that (in particular, see step 4).

6. Make sure that the project passes tests and compiles without compiler
warnings and clippy errors/warnings before opening a pull request/sending
a patch.
