import assert from "node:assert/strict";
import { test } from "node:test";
import { retainInstallerData } from "./prepare-windows-installer.mjs";

const template = [
  "Unicode true",
  '      ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"',
  '      ${IfThen} $UpdateMode = 1 ${|} StrCpy $R1 "$R1 /UPDATE" ${|} ; append /UPDATE',
  '      ${IfThen} $PassiveMode = 1 ${|} StrCpy $R1 "$R1 /P" ${|} ; append /P',
  '      StrCpy $R1 "$R1 _?=$4" ; append uninstall directory',
  "      ExecWait '$R1' $0",
  "Var DeleteAppDataCheckbox",
  "Var DeleteAppDataCheckboxState",
  "!define /ifndef WS_EX_LAYOUTRTL         0x00400000",
  "!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.ConfirmShow",
  "Function un.ConfirmShow ; Add add a `Delete app data` check box",
  "  ; $1 inner dialog HWND",
  "  ; $2 window DPI",
  "  ; $3 style",
  "  ; $4 x",
  "  ; $5 y",
  "  ; $6 width",
  "  ; $7 height",
  '  FindWindow $1 "#32770" "" $HWNDPARENT ; Find inner dialog',
  '  System::Call "user32::GetDpiForWindow(p r1) i .r2"',
  "  ${If} $(^RTL) = 1",
  '    StrCpy $3 "${__NSD_CheckBox_EXSTYLE} | ${WS_EX_LAYOUTRTL}"',
  "    IntOp $4 50 * $2",
  "  ${Else}",
  '    StrCpy $3 "${__NSD_CheckBox_EXSTYLE}"',
  "    IntOp $4 0 * $2",
  "  ${EndIf}",
  "  IntOp $5 100 * $2",
  "  IntOp $6 400 * $2",
  "  IntOp $7 25 * $2",
  "  IntOp $4 $4 / 96",
  "  IntOp $5 $5 / 96",
  "  IntOp $6 $6 / 96",
  "  IntOp $7 $7 / 96",
  '  System::Call \'user32::CreateWindowEx(i r3, w "${__NSD_CheckBox_CLASS}", w "$(deleteAppData)", i ${__NSD_CheckBox_STYLE}, i r4, i r5, i r6, i r7, p r1, i0, i0, i0) i .s\'',
  "  Pop $DeleteAppDataCheckbox",
  "  SendMessage $HWNDPARENT ${WM_GETFONT} 0 0 $1",
  "  SendMessage $DeleteAppDataCheckbox ${WM_SETFONT} $1 1",
  "FunctionEnd",
  "!define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.ConfirmLeave",
  "Function un.ConfirmLeave",
  "  SendMessage $DeleteAppDataCheckbox ${BM_GETCHECK} 0 0 $DeleteAppDataCheckboxState",
  "FunctionEnd",
  "!define MUI_PAGE_CUSTOMFUNCTION_PRE un.SkipIfPassive",
  "!insertmacro MUI_UNPAGE_CONFIRM",
  "",
  "; 2. Uninstalling Page",
  "!insertmacro MUI_UNPAGE_INSTFILES",
  "",
  "Section Uninstall",
  '  Delete "$INSTDIR\\socks-proxy.exe"',
  '  Delete "$DESKTOP\\Socks Proxy.lnk"',
  "  ; Delete app data if the checkbox is selected",
  "  ; and if not updating",
  "  ${If} $DeleteAppDataCheckboxState = 1",
  "  ${AndIf} $UpdateMode <> 1",
  "    ; Clear the install location $INSTDIR from registry",
  '    DeleteRegKey SHCTX "${MANUPRODUCTKEY}"',
  '    DeleteRegKey /ifempty SHCTX "${MANUKEY}"',
  "",
  "    ; Clear the install language from registry",
  '    DeleteRegValue HKCU "${MANUPRODUCTKEY}" "Installer Language"',
  '    DeleteRegKey /ifempty HKCU "${MANUPRODUCTKEY}"',
  '    DeleteRegKey /ifempty HKCU "${MANUKEY}"',
  "",
  "    SetShellVarContext current",
  '    RmDir /r "$APPDATA\\${BUNDLEID}"',
  '    RmDir /r "$LOCALAPPDATA\\${BUNDLEID}"',
  "  ${EndIf}",
  "",
  "  !ifmacrodef NSIS_HOOK_POSTUNINSTALL",
  "    !insertmacro NSIS_HOOK_POSTUNINSTALL",
  "  !endif",
  "",
  "SectionEnd",
  "",
].join("\n");

test("replacement always invokes the legacy NSIS uninstaller in data-preserving update mode", () => {
  const result = retainInstallerData(template);
  assert.match(result, /StrCpy \$R1 "\$R1 \/UPDATE"/);
  assert.doesNotMatch(result, /IfThen.*\/UPDATE/);
  assert.match(result, /\/UPDATE[\s\S]*_\?=\$4[\s\S]*ExecWait/);
});

test("standalone uninstall removes files and shortcuts while retaining both data directories", () => {
  const result = retainInstallerData(template);
  assert.doesNotMatch(result, /DeleteAppDataCheckbox|RmDir.*APPDATA/);
  assert.match(result, /代理配置和应用数据会保留/);
  for (const line of [
    '  Delete "$INSTDIR\\socks-proxy.exe"',
    '  Delete "$DESKTOP\\Socks Proxy.lnk"',
    "!insertmacro MUI_UNPAGE_CONFIRM",
    "    !insertmacro NSIS_HOOK_POSTUNINSTALL",
  ])
    assert.ok(result.includes(line));
});

test("changed or additional destructive template code is rejected", () => {
  assert.throws(() => retainInstallerData(template.replace("Var DeleteAppDataCheckbox\n", "")));
  assert.throws(() => retainInstallerData(template + '\nRmDir /r "$APPDATA\\other"'));
  assert.throws(() => retainInstallerData(template + template));
});
