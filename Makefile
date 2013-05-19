RUSTC=./rustc

all: main

%: %.rc
	$(RUSTC) $<

%-test: %.rs
	$(RUSTC) $< --test -o $@

test: intern-test op-test
	./intern-test
	./op-test

clean:
	$(RM) main *-test
