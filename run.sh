set -e
cargo run -- compile -s source.txt -o test-site/index.html && cat test-site/index.html | tidy --indent yes
