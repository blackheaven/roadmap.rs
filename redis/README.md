# Redis reimplementation

* https://codingchallenges.fyi/challenges/challenge-redis/
* https://tokio.rs/tokio/tutorial/spawning
* https://redis.io/docs/latest/develop/reference/protocol-spec/

## Benchmarks

```bash
#  redis 7.2.4
$ redis-benchmark -t set,get, -n 100000 -q
SET: 171232.88 requests per second, p50=0.151 msec
GET: 175746.92 requests per second, p50=0.151 msec

# rewrite (1st iteration)
$ redis-benchmark -t set,get, -n 100000 -q
SET: 69930.07 requests per second, p50=0.703 msec
GET: 67980.97 requests per second, p50=0.719 msec

# rewrite (2nd iteration: use RwLock)
SET: 67934.78 requests per second, p50=0.719 msec
GET: 174825.17 requests per second, p50=0.159 msec

# rewrite (3rd iteration: optimistic buffer buffer parsing)
SET: 72780.20 requests per second, p50=0.671 msec
GET: 177619.89 requests per second, p50=0.167 msec

# rewrite (4th iteration: drop locks, use mpsc)
SET: 160513.64 requests per second, p50=0.175 msec
GET: 175131.36 requests per second, p50=0.167 msec

# rewrite (5th iteration: extract and replicate kv store)
SET: 161812.31 requests per second, p50=0.175 msec
GET: 161812.31 requests per second, p50=0.175 msec

```
