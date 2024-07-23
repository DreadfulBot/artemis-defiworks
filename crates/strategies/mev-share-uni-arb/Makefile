include .env

build:
	forge build \
		--root ./contracts
test:
	forge test \
		--root ./contracts \
		-vv \
		-w \
		--gas-report
make-bindings:
	forge bind --bindings-path ./bindings \
		--root ./contracts \
		--crate-name bindings 