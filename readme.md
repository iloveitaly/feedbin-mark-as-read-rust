# Automatically Mark Articles Older Than a Month as Read in Feedbin

Rewrite of https://github.com/iloveitaly/feedbin-mark-as-read-clojure in rust. Good excuse to learn rust using ChatGPT.

Too many RSS articles to read stresses me out. Auto-removing stuff you don't want to see eliminates noise. More info on this original blog post:

https://mikebian.co/learning-clojure-with-feed-automation

## Development

```shell
cargo run -- --dry-run
```

Or, using docker:

```shell
docker build . -t feedbin-run

# docker doesn't like `export ` but direnv does
sed 's/^export //' ".envrc" > .env

docker run --env-file .env -it feedbin-run
```
