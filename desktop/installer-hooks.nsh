!macro NSIS_HOOK_POSTINSTALL
  ReadEnvStr $1 "ProgramData"
  CreateDirectory "$1\Mad Library Server"
  nsExec::ExecToLog '"$SYSDIR\icacls.exe" "$1\Mad Library Server" /grant "*S-1-5-32-545:(OI)(CI)M" /T /C /Q'
  Pop $0
  StrCmp $0 "0" madlibrary_permissions_ready madlibrary_permissions_done
madlibrary_permissions_ready:
  FileOpen $0 "$1\Mad Library Server\.machine-permissions-v1" w
  FileWrite $0 "1$\r$\n"
  FileClose $0
madlibrary_permissions_done:
!macroend
