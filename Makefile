PKG := ldoce5viewer
# Allow overriding which Python interpreter to use (e.g. your mise/python)
PYTHON ?= python

# Build the application. Use "python -m PyInstaller" so the command runs in the
# same Python environment as $(PYTHON). This avoids failures when the
# "pyinstaller" script is not on PATH but the package is installed in the
# selected interpreter.
build: clean precompile
	$(PYTHON) -m PyInstaller ldoce5viewer.spec

install: build
	$(PYTHON) ./setup.py install
	cp ./ldoce5viewer.desktop /usr/share/applications/
	cp ./ldoce5viewer/qtgui/resources/ldoce5viewer.svg /usr/share/pixmaps/
	[ -x /usr/bin/update-desktop-database ] && sudo update-desktop-database -q

sdist: precompile
	$(PYTHON) ./setup.py sdist

precompile: qtui qtresource

qtui:
	cd $(PKG)/qtgui/ui/; $(MAKE)

qtresource:
	cd $(PKG)/qtgui/resources/; $(MAKE)

.PHONY: clean clean-build
clean: clean-build
	cd $(PKG)/qtgui/ui/; $(MAKE) clean
	cd $(PKG)/qtgui/resources/; $(MAKE) clean

clean-build:
	rm -rf build
	rm -rf dist
	rm -f MANIFEST

# Create a macOS DMG. Depends on the built .app (build target creates the .app).
# This target uses the create-dmg helper if available; otherwise falls back to hdiutil.
dmg: build
	# Prepare staging directory
	mkdir -p dist/dmg
	# Copy the app bundle to the staging folder
	cp -r "dist/LDOCE5 Viewer.app" dist/dmg/
	# Prefer create-dmg if installed
	if command -v create-dmg >/dev/null 2>&1; then \
		create-dmg \
		  --volname "LDOCE5 Viewer" \
		  --volicon "./ldoce5viewer/qtgui/resources/ldoce5viewer.icns" \
		  --window-pos 200 120 \
		  --window-size 600 300 \
		  --icon-size 100 \
		  --icon "LDOCE5 Viewer.app" 175 120 \
		  --hide-extension "LDOCE5 Viewer.app" \
		  --app-drop-link 425 120 \
		  "dist/LDOCE5 Viewer.dmg" \
		  "dist/dmg/"; \
	else \
		echo "create-dmg not found, using hdiutil fallback"; \
		rm -f "dist/LDOCE5 Viewer.dmg" "dist/tmp.dmg"; \
		hdiutil create -srcfolder "dist/dmg" -volname "LDOCE5 Viewer" -fs HFS+ -format UDRO "dist/tmp.dmg"; \
		hdiutil convert "dist/tmp.dmg" -format UDZO -imagekey zlib-level=9 -o "dist/LDOCE5 Viewer.dmg"; \
		rm -f "dist/tmp.dmg"; \
	fi
