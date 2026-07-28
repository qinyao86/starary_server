# Starary Server 浜ゆ帴鏂囨。

鏈€鍚庢洿鏂帮細2026-07-15

## 1. 椤圭洰瀹氫綅

鏈粨搴撳悓鏃跺寘鍚?Starary 鍥㈤槦鏈嶅姟绔€佹祻瑙堝櫒绠＄悊鍚庡彴鍜?Windows Tauri 妗岄潰澹炽€?

- Rust 鏈嶅姟绔彁渚?HTTP API銆佺鐞嗗悗鍙伴潤鎬佹枃浠躲€侀壌鏉冦€佽祫婧愬簱銆佹垚鍛樸€佸偍瀛樸€佸浠藉拰杩愯鎺у埗銆?
- React 绠＄悊鍚庡彴鍚屾椂杩愯鍦ㄦ闈?WebView 鍜屾櫘閫氭祻瑙堝櫒涓€?
- Windows 妗岄潰澹虫槸鎷ユ湁鑰呬娇鐢ㄧ殑鍚姩鍣ㄥ拰鏈満绠＄悊鍏ュ彛锛屼笉鏄敮涓€绠＄悊鍏ュ彛銆?
- 妗岄潰瀹夎鐗堝惎鍔ㄧ鏈?PostgreSQL 涓?Rust 鏈嶅姟锛涘鎴风鍜屽眬鍩熺綉娴忚鍣ㄥ彧璁块棶 HTTP 鏈嶅姟锛屼笉鐩存帴璁块棶鏁版嵁搴撱€?
- 榛樿 HTTP 鍦板潃涓?`0.0.0.0:3789`锛岀鏈?PostgreSQL 浠呯洃鍚?`127.0.0.1:54329`銆?

棣栦釜 Owner 鍙兘浠庢湇鍔＄涓绘満鐨勫洖鐜湴鍧€鍒涘缓銆傚垵濮嬪寲瀹屾垚鍚庯紝宸叉巿鏉冪鐞嗗憳鍙粠灞€鍩熺綉娴忚鍣ㄨ闂悗鍙般€?

## 2. 鎶€鏈爤

- 鏈嶅姟鏍稿績锛歊ust銆丄xum銆乀okio銆丼QLx銆丳ostgreSQL銆丣WT
- 绠＄悊鍚庡彴锛歊eact銆乀ypeScript銆乂ite銆丩ucide React
- 妗岄潰澹筹細Tauri 2銆丷ust銆丯SIS
- Windows 鏁版嵁搴擄細浠撳簱鍐呯簿绠€ PostgreSQL x64 杩愯鏃?
- 浜岃繘鍒剁増鏈鐞嗭細Git LFS

## 3. 閲嶈鐩綍

```text
madlibrary-server/
|- src/                         Rust 鏈嶅姟绔笌鏁版嵁搴撹縼绉?
|- admin-ui/src/                绠＄悊鍚庡彴婧愮爜
|- desktop/                     Tauri 妗岄潰澹虫簮鐮佸拰瀹夎閰嶇疆
|- binaries/windows-x64/        闅忎唬鐮佽窡韪殑绮剧畝 PostgreSQL 杩愯鏃?
|- packaging/                   渚挎惡鍖呭畾涔夊拰绗笁鏂硅繍琛屾椂娓呭崟
|- scripts/                     寮€鍙戙€佸噯澶囪繍琛屾椂鍜屽彂甯冭剼鏈?
|- docs/                        鏋舵瀯銆佺洰褰曞拰浜ゆ帴鏂囨。
|- target/                      缂栬瘧涓庢闈㈠紑鍙戣繍琛屾椂锛屼笉鎻愪氦
|- target/build/frontend/admin-ui/               Vite 鏋勫缓杈撳嚭锛屼笉鎻愪氦
`- target/release/windows-x64/       鏈€缁堝畨瑁呭寘鍜屾牎楠屾枃浠讹紝涓嶆彁浜?
```

`desktop/bundle-resources/runtime/` 鍙槸 Tauri 鎵撳寘涓棿鐩綍銆傞櫎 `.gitkeep` 澶栧潎蹇界暐锛屽彂甯冭剼鏈粠婧愮爜鍜?`binaries/` 閲嶆柊鐢熸垚鍐呭銆備笉瑕佹妸瀹夎鍚庣殑鏁版嵁搴撴垨娴嬭瘯鏁版嵁鏀惧叆浠撳簱銆?

## 4. 鏈湴寮€鍙?

鐜瑕佹眰锛歂ode.js/npm銆佺ǔ瀹氱増 Rust/MSVC銆丟it LFS銆傛甯稿紑鍙戝拰鎵撳寘涓嶉渶瑕佹湰鏈哄畨瑁?PostgreSQL銆?

棣栨鎷夊彇鍚庢墽琛岋細

```powershell
git lfs pull
cd .\admin-ui
npm ci
cd ..\desktop
npm ci
cd ..
```

鍚姩瀹屾暣妗岄潰寮€鍙戠増锛?

```powershell
npm run desktop:dev
```

璇ュ懡浠や細鏋勫缓绠＄悊鍚庡彴銆佸噯澶?`target/build-dev/desktop/runtime/`锛岀劧鍚庡惎鍔?Tauri銆備笉瑕佸悓鏃跺啀杩愯涓€浠?`cargo run`锛屽惁鍒欎細浜夌敤绔彛鎴栧疄渚嬮攣銆?
鏅鸿兘浣撴敞鎰忥細濡傛灉鍙湪 `admin-ui/` 涓嬫墽琛?`npm run build`锛孷ite 鍙細鏇存柊 `target/build/frontend/admin-ui/`銆傛闈㈠紑鍙戣繍琛屾椂鍜?`http://127.0.0.1:3789/admin/` 瀹為檯璇诲彇鐨勬槸 `target/build-dev/desktop/runtime/admin-ui/` 鐨勫鍒跺搧銆傛瘡娆″笇鏈涙闈㈢鎴?3789 鍚庡彴鐪嬪埌鏈€鏂板墠绔椂锛屾瀯寤哄悗蹇呴』鍚屾涓€娆★細

```powershell
robocopy target\build\frontend\admin-ui target\build-dev\desktop\runtime\admin-ui /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP
if ($LASTEXITCODE -gt 7) { exit $LASTEXITCODE }
```

涔熷彲浠ョ洿鎺ヨ繍琛?`npm run desktop:dev`锛岃鑴氭湰浼氭瀯寤哄苟鍑嗗 `target/build-dev/desktop/runtime/`銆傝嫢鍚屾鍚庨〉闈粛鏃э紝寮哄埛娴忚鍣ㄦ垨閲嶅惎妗岄潰绐楀彛锛岃瀹冮噸鏂板姞杞芥柊鐨?Vite hash 璧勬簮銆?
鍙紑鍙戠鐞嗗悗鍙版椂锛屽彲鍏堝惎鍔ㄦ湇鍔＄锛屽啀杩愯 Vite锛?
```powershell
cargo run --manifest-path .\Cargo.toml
cd .\admin-ui
npm run dev
```

Vite 榛樿鍦?`http://127.0.0.1:5179/admin/`锛屽苟灏?API 浠ｇ悊鍒?`http://127.0.0.1:3789`銆?

## 5. 杩愯妯″瀷涓庢暟鎹洰褰?

妗岄潰瀹夎鐗堢殑鍙墽琛屾枃浠跺拰 PostgreSQL 绋嬪簭鏂囦欢浣嶄簬瀹夎鐩綍锛屽彧璇昏繍琛屾暟鎹粺涓€浣嶄簬锛?

```text
C:\ProgramData\Mad Library Server\
|- data\config\runtime.json     鏈嶅姟绔彛銆佹暟鎹簱闅忔満瀵嗙爜鍜?JWT 瀵嗛挜
|- data\config\backup.json      鑷姩澶囦唤璁剧疆
|- data\postgresql\             PostgreSQL 鏁版嵁绨?
|- data\storage\                榛樿鏂囦欢鍌ㄥ瓨鐩綍
|- data\backups\                PostgreSQL 鑷畾涔夋牸寮忓浠芥枃浠?
|- data\logs\postgresql.log     PostgreSQL 鏃ュ織
`- logs\server.log              妗岄潰澹冲惎鍔ㄧ殑鏈嶅姟绔棩蹇?
```

杩欐槸鏈哄櫒绾ф暟鎹洰褰曪紝鍥犳妗岄潰澹冲拰鏈嶅姟鏍稿績閮介€氳繃瀹炰緥閿佷繚璇佸悓涓€鏁版嵁鐩綍鍙湁涓€涓湇鍔″疄渚嬨€傚嵏杞芥垨鍗囩骇绋嬪簭鏃朵笉寰楄嚜鍔ㄥ垹闄よ鐩綍銆?

寮€鍙戠増鐢?`MADLIBRARY_HOME` 鍐冲畾鏁版嵁鐩綍锛涙闈㈠紑鍙戣剼鏈娇鐢?`target/build-dev/desktop/runtime` 浣滀负绋嬪簭杩愯鏃讹紝浣嗘寔涔呮暟鎹竟鐣屼粛鐢辨湇鍔￠厤缃喅瀹氥€傛帓闅滄椂鍏堢湅瀹為檯杩涚▼鐜鍜屾棩蹇楋紝涓嶈鎶?`target/` 褰撲綔鍙戝竷鐗┿€?

## 6. PostgreSQL 妯″紡

`MADLIBRARY_POSTGRES_MODE` 鏀寔锛?

- `auto`锛氶粯璁ゆā寮忥紱璁剧疆浜?`MADLIBRARY_DATABASE_URL` 鏃朵娇鐢ㄥ閮ㄦ暟鎹簱锛屽惁鍒欏惎鍔ㄩ殢绋嬪簭闄勫甫鐨?PostgreSQL銆?
- `bundled`锛氬繀椤诲惎鍔ㄩ檮甯︾殑 PostgreSQL銆?
- `external`锛氫笉鍚姩闄勫甫鏁版嵁搴擄紝蹇呴』鎻愪緵 `MADLIBRARY_DATABASE_URL`銆?

`MADLIBRARY_SERVER_PORT` 鍙复鏃惰鐩栫鍙ｃ€傚畨瑁呯増鍦ㄨ缃〉淇敼绔彛鍚庡啓鍏?`data/config/runtime.json`锛岄噸鍚敓鏁堛€傚眬鍩熺綉閮ㄧ讲杩橀渶鍦?Windows 绉佹湁/鍩熺綉缁滈槻鐏涓斁琛岄€夊畾 HTTP 绔彛锛岀粷涓嶈兘鏀捐 `54329`銆?

## 7. 褰撳墠涓氬姟绾︽潫

- 鐢ㄦ埛鍙兘鐪嬪埌鑷繁琚姞鍏ョ殑璧勬簮搴擄紱鏈嶅姟绔鑹蹭笌璧勬簮搴撹鑹插垎寮€淇濆瓨銆?
- 璧勬簮搴撳彲鍏抽棴銆傚叧闂悗瀹㈡埛绔?API 缁熶竴杩斿洖 `library_disabled`锛屽鎴风鎹杩涘叆鏆傚仠椤甸潰骞惰疆璇㈡仮澶嶇姸鎬併€?
- 姣忎釜璧勬簮搴撶粦瀹氫竴涓渶缁堝偍瀛樼洰褰曘€備笉鍚岃祫婧愬簱鐨勬渶缁堢洰褰曚笉寰楃浉鍚岋紝涔熶笉寰椾簰涓虹埗瀛愮洰褰曘€?
- 绌鸿祫婧愬簱鍙互鐩存帴鏇存崲鍌ㄥ瓨锛涘凡鏈夎祫婧愬紩鐢ㄥ偍瀛樺悗锛屼笉搴旂洿鎺ユ崲缁戯紝鏈潵闇€璧板鍒躲€佹牎楠屻€佸垏鎹㈠拰鍥炴粴鐨勮縼绉绘祦绋嬨€?
- 璧勬簮搴撳垹闄や粎绠＄悊鍛樺彲鎵ц锛屽苟鍙€夋嫨鍚屾椂鍒犻櫎鏂囦欢銆傚垹闄ゆ枃浠跺睘浜庨珮椋庨櫓璺緞锛屽繀椤荤户缁繚鎸佹潈闄愪笌璺緞杈圭晫妫€鏌ャ€?
- 鏁版嵁椤垫敮鎸佹瘡鏃ヨ嚜鍔ㄥ浠姐€佹墜鍔ㄥ浠姐€佷笅杞姐€佸垹闄ゃ€佹仮澶嶅拰鏈嶅姟鍣ㄥ垵濮嬪寲銆?
- 鑷姩澶囦唤榛樿姣忓ぉ `02:00` 鎵ц骞朵繚鐣?30 浠斤紝鍙厤缃寖鍥翠负 1 鑷?365銆?
- 鎭㈠鍓嶅拰鍒濆鍖栧墠浼氬垱寤哄畨鍏ㄥ浠姐€傛仮澶嶉€氳繃閲嶅惎鏃跺簲鐢ㄥ緟鎭㈠璁板綍瀹屾垚銆?
- 鍙湁 Owner 鍙互鍒濆鍖栨湇鍔″櫒锛涘垵濮嬪寲浼氭竻绌轰笟鍔℃暟鎹苟鍥炲埌棣栨娆㈣繋娴佺▼銆?

## 8. 绠＄悊鍚庡彴缁撴瀯

`admin-ui/src/App.tsx` 璐熻矗閴存潈銆佸垵濮嬪寲娴佺▼銆佽矾鐢辩姸鎬佸拰鍏ㄥ眬鏁版嵁鍒锋柊銆傛寮忓悗鍙板叕鍏辨鏋朵綅浜庯細

- `components/admin-shell.tsx`锛氫晶鏍忋€佹爣棰樺尯銆佸唴瀹瑰尯鍜岄€€鍑哄叆鍙?
- `components/desktop-titlebar.tsx`锛歍auri 鑷畾涔夋爣棰樻爮銆佸墠杩涘悗閫€銆佹姌鍙犲拰绐楀彛鎸夐挳
- `pages/`锛氳祫婧愬簱銆佺敤鎴枫€佸偍瀛樸€佺粺璁°€佹暟鎹拰璁剧疆
- `components/dialogs/`锛氱紪杈戙€佸垹闄ゃ€佹仮澶嶃€佸垵濮嬪寲鍜屽叧鏈虹‘璁?
- `api/`锛氳姹傘€佺被鍨嬪拰绔偣杈圭晫

璺敱浣跨敤娴忚鍣?History API銆傞〉闈㈠垏鎹細璋冪敤鏁版嵁鍒锋柊鍑芥暟锛屼絾涓嶄細寮哄埗鏁撮〉閲嶈浇锛涚偣鍑诲綋鍓嶄晶鏍忛」鐩篃浼氬埛鏂板綋鍓嶆暟鎹€傛柊澧為〉闈㈠簲澶嶇敤鐜版湁椤甸潰鏍囬鍜?`Panel`/鍗＄墖鏍峰紡锛屼笉瑕佸彟寤轰竴濂楅〉闈㈠３銆?

## 9. 鏋勫缓涓庡彂甯?

鏋勫缓 Windows 瀹夎鐗堬細

```powershell
npm run release:windows
```

鏈€缁堝彧浠庝互涓嬬洰褰曞彇鍙戝竷鐗╋細

```text
target/release/windows-x64/
|- Starary-Server_0.1.0_windows-x64-setup.exe
`- SHA256SUMS.txt
```

NSIS 閰嶇疆涓?`installMode: both`锛屽厑璁哥敤鎴烽€夋嫨浠呭綋鍓嶇敤鎴锋垨鎵€鏈夌敤鎴峰畨瑁呫€傚彂甯冨墠蹇呴』纭 `pg_dump.exe`銆乣pg_restore.exe` 鍜?`dropdb.exe` 瀛樺湪浜庣簿绠€ PostgreSQL `bin/` 涓紝鍚﹀垯澶囦唤銆佹仮澶嶆垨鍒濆鍖栨祦绋嬩笉瀹屾暣銆?

鏃犵晫闈㈡湇鍔″櫒浠嶅彲浣跨敤 `scripts/build-windows-portable.ps1` 鏋勫缓渚挎惡鍖呫€傛湭鏉ュ叕缃戦儴缃插缓璁娇鐢?Linux OCI 瀹瑰櫒銆佸閮?PostgreSQL銆佸璞″偍瀛樺拰鍙嶅悜浠ｇ悊 TLS锛汥ocker 涓嶆槸 Windows 妗岄潰瀹夎鐗堢殑杩愯渚濊禆銆?

## 10. 鎻愪氦鍓嶉獙璇?

```powershell
cd .\admin-ui
npm run build
cd ..
cargo test --locked
cargo fmt --check
$env:CARGO_TARGET_DIR = (Join-Path (Get-Location) 'target\desktop')
cargo check --locked --manifest-path .\desktop\Cargo.toml
git diff --check
```

娑夊強妗岄潰澹炽€佺鍙ｃ€佸惎鍔ㄦ垨鍏抽棴閫昏緫鏃讹紝杩樺簲鎵嬪伐楠岃瘉锛氬崟瀹炰緥銆佹祻瑙堝櫒璁块棶銆佸眬鍩熺綉鍦板潃銆佸叧闂獥鍙ｅ悗瀛愯繘绋嬮€€鍑恒€佺鍙ｉ噴鏀俱€侀噸鍚悗鏂扮鍙ｇ敓鏁堛€?

## 11. 宸茬煡椋庨櫓涓庡悗缁伐浣?

- 鍙戣鐗堝皻闇€浠ｇ爜绛惧悕銆佸崌绾х瓥鐣ュ拰绗笁鏂硅鍙瘉瀹¤銆?
- 鍟嗕笟甯綅鎺堟潈銆侀個璇峰埗璐﹀彿鐢熷懡鍛ㄦ湡鍜屽璁′簨浠朵粛闇€瀹屾垚銆?
- 鍌ㄥ瓨杩佺Щ銆丼3 鍏煎瀵硅薄鍌ㄥ瓨涓庡鎴风鍚屾鐩綍灏氭湭瀹炵幇銆?
- 鍏綉閮ㄧ讲娓呭崟銆乀LS銆佸弽鍚戜唬鐞嗗拰澶栭儴鏁版嵁搴撹繍缁翠粛闇€琛ラ綈銆?
- README 涓儴鍒嗏€滈鐗堣寖鍥粹€濇弿杩板彲鑳借惤鍚庝簬褰撳墠澶囦唤鍔熻兘锛涗慨鏀瑰姛鑳芥椂搴斿悓姝?README銆佽矾绾垮浘鍜屾湰鏂囨。銆?
- 妗岄潰澹冲叧闂簲浼樺厛璋冪敤鍙楁帶鍒朵护鐗屼繚鎶ょ殑浼橀泤鍏抽棴鎺ュ彛锛涜嫢鍐嶆鍑虹幇 PostgreSQL 娈嬬暀杩涚▼锛屽厛淇杩涚▼鐢熷懡鍛ㄦ湡锛屼笉瑕佺敤鍏ㄥ眬杩涚▼娓呯悊鎺╃洊闂銆?
