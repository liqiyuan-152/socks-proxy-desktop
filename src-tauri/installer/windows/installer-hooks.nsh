!include "${__FILEDIR__}\notice-page.nsh"

; Windows may retain the old icon cached for an unchanged executable path.
; Give shortcuts a separate icon source without replacing their other properties.
!macro SetBrandShortcutIcon shortcut
  !define BrandIconUniqueID ${__COUNTER__}
  IfFileExists "$INSTDIR\brand-shield.ico" 0 brand_icon_done_${BrandIconUniqueID}
  !insertmacro IsShortcutTarget "${shortcut}" "$INSTDIR\socks-proxy.exe"
  Pop $0
  ${If} $0 = 1
    !insertmacro ComHlpr_CreateInProcInstance ${CLSID_ShellLink} ${IID_IShellLink} r0 ""
    ${If} $0 P<> 0
      ${IUnknown::QueryInterface} $0 '("${IID_IPersistFile}",.r1)'
      ${If} $1 P<> 0
        ${IPersistFile::Load} $1 '("${shortcut}", ${STGM_READWRITE})'
        ${IShellLink::SetIconLocation} $0 '(w "$INSTDIR\brand-shield.ico", 0)'
        ${IPersistFile::Save} $1 '("${shortcut}",1)'
        ${IUnknown::Release} $1 ""
        System::Call 'shell32::SHChangeNotify(i 0x2000, i 0x5, w "${shortcut}", p 0)'
      ${EndIf}
      ${IUnknown::Release} $0 ""
    ${EndIf}
  ${EndIf}
  brand_icon_done_${BrandIconUniqueID}:
  !undef BrandIconUniqueID
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Push $0
  Push $1
  Push $2
  Push $3
  !insertmacro SetBrandShortcutIcon "$DESKTOP\${PRODUCTNAME}.lnk"
  !insertmacro SetBrandShortcutIcon "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !insertmacro SetBrandShortcutIcon "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend

; The official finish page creates the optional desktop shortcut after POSTINSTALL.
; Do not create an unchecked shortcut; update only a link targeting this install.
Function .onGUIEnd
  Push $0
  Push $1
  Push $2
  Push $3
  !insertmacro SetBrandShortcutIcon "$DESKTOP\Socks Proxy.lnk"
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd
