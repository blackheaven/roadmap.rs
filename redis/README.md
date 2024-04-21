# Redis reimplementation

* https://codingchallenges.fyi/challenges/challenge-redis/
* https://tokio.rs/tokio/tutorial/spawning
* https://redis.io/docs/latest/develop/reference/protocol-spec/

## Benchmarks

```bash
# rewrite
$ redis-benchmark -t set,get, -n 100000 -q
SET: 69930.07 requests per second, p50=0.703 msec
GET: 67980.97 requests per second, p50=0.719 msec

#  redis 7.2.4
$ redis-benchmark -t set,get, -n 100000 -q
SET: 171232.88 requests per second, p50=0.151 msec
GET: 175746.92 requests per second, p50=0.151 msec
```
