# Demo — devcontainer-env CLI walkthrough

Output doc/tapes/output/demo.webp

Set Shell "zsh"
Set FontSize 18
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set FontFamily "JetBrainsMono Nerd Font"
Set Framerate 15
Set CursorBlink false
Set Padding 20
Set TypingSpeed 75ms
Hide
Type "cd example && clear"
Enter
Sleep 500ms
Type "devcontainer up --workspace-folder ."
Enter
Sleep 15s
Show

Type "devcontainer-env --version"
Enter
Sleep 1s
Type "clear"
Enter
Sleep 500ms

Type "devcontainer-env --help"
Enter
Sleep 2s
Type "clear"
Enter
Sleep 500ms

Type "devcontainer-env inspect"
Enter
Sleep 2s
Type "clear"
Enter
Sleep 500ms

Type "devcontainer-env export"
Enter
Sleep 2s
Type "clear"
Enter
Sleep 500ms

Type "devcontainer-env export --format json"
Enter
Sleep 2s
Type "clear"
Enter
Sleep 500ms

Type "devcontainer-env exec -- env | grep EXAMPLE_API"
Enter
Sleep 2s
Type "clear"
Enter
Sleep 500ms

Hide
Type "devcontainer down --workspace-folder ."
Enter
Sleep 5s
Show
