# Terminal recording of the Spreadsheet analyst; from the repository root:
# `vhs docs/demo/spreadsheet-analyst.tape`
# (https://github.com/charmbracelet/vhs). Needs kowalski running with tableski connected and
# tableski serving demo/sample.xlsx; set KOWALSKI_URL before recording if not the default.
# The minutes the model spends are cut from the recording.

Output docs/img/spreadsheet-analyst.gif

Set Shell bash
Set FontSize 15
Set Width 1100
Set Height 720
Set Padding 20
Set TypingSpeed 35ms
Set WaitTimeout 600s

Hide
Type "cd examples/spreadsheet-analyst && clear"
Enter
Show

Type "./demo.sh 'Who spent the most in total across the orders?' 'How many people are active?'"
Sleep 500ms
Enter

Wait+Screen /profile:success/
Sleep 1500ms
Hide
Wait+Screen /Workbook:/
Show
Sleep 8s
