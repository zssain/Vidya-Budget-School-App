; NSIS installer hooks (prompts/P09 §2/§8). Wired via
; tauri.conf.json → bundle.windows.nsis.installerHooks.
;
; On install, add a Windows Firewall rule allowing Vidya to accept inbound
; connections on the PRIVATE profile only (the school LAN) — never on Public.
; On uninstall, remove the rule. The rule is scoped to this install's Vidya.exe.
;
; Data is NOT touched here: the default Tauri uninstaller removes only the
; program files, leaving the school database and backups in %APPDATA%\Vidya
; intact unless the user ticks "Remove school data" (see docs/phase-notes/phase-9.md
; for the pending custom-uninstall-page work).

!macro NSIS_HOOK_POSTINSTALL
  ; Remove any stale rule first so re-installs don't stack duplicates.
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Vidya (school LAN)"'
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="Vidya (school LAN)" dir=in action=allow program="$INSTDIR\Vidya.exe" enable=yes profile=private description="Allow Vidya to run the school server on the private (LAN) network."'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Vidya (school LAN)"'
!macroend
