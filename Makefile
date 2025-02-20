dist/yaml-split-%.tar.gz: dist/yaml-split-%
	tar -czvf $@ -C $(<D) $(<F)

.PRECIOUS: dist/yaml-split-%

dist/yaml-split-%: doc/RELEASE-README.txt doc/yaml-split.1 doc/LICENSES.html target/%/release-opt/yaml-split
	rm -rf $@
	mkdir -p $@
	cp $^ $@/

doc/LICENSES.html: Cargo.lock about.toml about.hbs
	cargo about generate about.hbs > $@

target/%/release-opt/yaml-split:
	cargo build --profile release-opt --target $*

.PHONY: clean

clean:
	rm -rf dist target doc/LICENSES.html
