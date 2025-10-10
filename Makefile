setup:
	cargo install --locked --no-default-features --features=native-tls --git https://github.com/getzola/zola
	npm install tailwindcss @tailwindcss/cli @tailwindcss/typography

build:
	cargo jobgen
	cargo eventgen
	zola build
	npx @tailwindcss/cli -m --optimize -i styles/tailwind.css -o public/main.t.css

serve:
	zola serve & npx @tailwindcss/cli -w -i styles/tailwind.css -o public/main.t.css
