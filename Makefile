doc/LICENSES.html: Cargo.lock about.hbs
	cargo about generate about.hbs > $@
