count=$1

for case in `cargo fuzz list`
do
	cargo +nightly fuzz run ${case} --dev -- -runs=${count}
done
