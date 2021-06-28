set -e
cargo run -- compile \
    -s examples/source/electrical-engineering.txt \
    -o test-site/index.html

cat test-site/index.html | tidy -quiet --indent yes


