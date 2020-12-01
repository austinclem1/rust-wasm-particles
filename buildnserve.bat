rmdir pkg /s /q
rmdir www/dist /s /q
wasm-pack build
cd www
npm run start