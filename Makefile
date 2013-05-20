RUSTC=./rustc

all: main

%: %.rc
	$(RUSTC) $<

%-test: %.rc
	$(RUSTC) $< --test -o $@
# useful for intern-test, op-test, etc.
%-test: %.rs
	$(RUSTC) $< --test -o $@

test: main-test
	./main-test

clean:
	$(RM) main *-test

# extra dependencies
main main-test: *.rs
