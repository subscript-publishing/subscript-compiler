set -e

cargo run --example html > test-site/index.html

cat test-site/index.html | tidy -quiet --indent yes
