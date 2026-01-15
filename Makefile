setup:
	cargo install --locked --no-default-features --features=native-tls --git https://github.com/getzola/zola --tag v0.21.0
	npm install tailwindcss @tailwindcss/cli @tailwindcss/typography @fontsource/fira-sans @fontsource/source-serif-4

build:
	cargo jobgen
	cargo eventgen
	zola build
	npx @tailwindcss/cli -m -i styles/tailwind.css -o public/main.t.css
	mkdir -p public/files
	cp node_modules/@fontsource/fira-sans/files/*-latin-*.woff2 public/files/
	cp node_modules/@fontsource/source-serif-4/files/*-latin-*.woff2 public/files/

serve: build
	zola serve & npx @tailwindcss/cli -w -i styles/tailwind.css -o public/main.t.css
