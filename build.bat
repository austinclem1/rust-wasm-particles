rmdir pkg /s /q
rmdir www\dist /s /q
wasm-pack build
@REM cd www
@REM npm run build
