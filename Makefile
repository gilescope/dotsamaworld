./js/bundle.js: ./js/src/index.js
	# browserify ./js/src/index.js -o ./js/bundle.js
	./js/node_modules/.bin/esbuild js/src/index.js --bundle --format=iife --global-name=xyz --outfile=js/bundle.js