!macro NSIS_HOOK_POSTINSTALL
  ReadEnvStr $1 "ProgramData"
  CreateDirectory "$1\Starary Server"
  nsExec::ExecToLog '"$SYSDIR\icacls.exe" "$1\Starary Server" /grant "*S-1-5-32-545:(OI)(CI)M" /T /C /Q'
  Pop $0
  StrCmp $0 "0" starary_permissions_ready starary_permissions_done
starary_permissions_ready:
  FileOpen $0 "$1\Starary Server\.machine-permissions-v1" w
  FileWrite $0 "1$\r$\n"
  FileClose $0
starary_permissions_done:
!macroend
