PGRX_POSTGRES ?= pg16
DISTNAME = $(shell grep -m 1 '^name' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')
DISTVERSION  = $(shell grep -m 1 '^version' Trunk.toml | sed -e 's/[^"]*"\([^"]*\)",\{0,1\}/\1/')

META.json: META.json.in Cargo.toml
	@sed "s/@CARGO_VERSION@/$(DISTVERSION)/g" $< > $@

$(DISTNAME)-$(DISTVERSION).zip: META.json
	git archive --format zip --prefix $(DISTNAME)-$(DISTVERSION)/ --add-file $< -o $(DISTNAME)-$(DISTVERSION).zip HEAD

pgxn-zip: $(DISTNAME)-$(DISTVERSION).zip

clean:
	@rm -rf META.json $(DISTNAME)-$(DISTVERSION).zip