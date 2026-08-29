# The Machine — Documentation Build

.PHONY: html serve clean

html:
	python3 build.py

serve: html
	cd build && python3 -m http.server 8000

clean:
	rm -rf build
