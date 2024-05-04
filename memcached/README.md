# Redis reimplementation

* https://codingchallenges.fyi/challenges/challenge-redis/
* https://tokio.rs/tokio/tutorial/spawning
* https://redis.io/docs/latest/develop/reference/protocol-spec/

## Benchmarks

```bash
#  redis 7.2.4
$ redis-benchmark -t set,get, -n 1000000 -q
SET: 183049.61 requests per second, p50=0.143 msec
GET: 183183.73 requests per second, p50=0.143 msec


# rewrite (1st iteration)
$ redis-benchmark -t set,get, -n 1000000 -q
SET: 67971.73 requests per second, p50=0.719 msec
GET: 66769.05 requests per second, p50=0.735 msec

# rewrite (2nd iteration: use RwLock) (old bench)
SET: 67934.78 requests per second, p50=0.719 msec
GET: 174825.17 requests per second, p50=0.159 msec

# rewrite (3rd iteration: optimistic buffer buffer parsing)
SET: 164068.92 requests per second, p50=0.175 msec
GET: 163880.69 requests per second, p50=0.167 msec

# rewrite (4th iteration: drop locks, use mpsc)
SET: 162786.91 requests per second, p50=0.175 msec
GET: 163719.72 requests per second, p50=0.175 msec

# rewrite (5th iteration: extract and replicate kv store)
SET: 159642.41 requests per second, p50=0.175 msec
GET: 163773.34 requests per second, p50=0.175 msec

# rewrite (6th iteration: rewrite buffer read access)
SET: 147470.88 requests per second, p50=0.191 msec
GET: 154392.45 requests per second, p50=0.183 msec
```
