Output assets/zatsu-demo.gif

Require zatsu

Set Shell "bash"
Set Theme "Catppuccin Mocha"
Set FontFamily "Menlo"
Set FontSize 22
Set LetterSpacing 0
Set Width 1000
Set Height 340
Set Padding 20
Set TypingSpeed 35ms
Set Framerate 30

Hide
Type "export PS1=$'\\e[38;5;141m>\\e[0m '"
Enter
Type "clear"
Enter
Show

Sleep 500ms
Type "zatsu tests/fixtures/repository"
Sleep 300ms
Enter
Sleep 4s
