```sh
$ wrk -t8 -c100 -d30s "http://127.0.0.1:42069"
```

# Debug - Full Logs

```
Running 30s test @ http://127.0.0.1:42069
  8 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     4.74ms    1.63ms  18.10ms   68.56%
    Req/Sec     2.55k   142.25     4.05k    81.00%
  608346 requests in 30.02s, 81.22MB read
Requests/sec:  20264.80
Transfer/sec:      2.71MB
```

# Debug - Less Logs

```
Running 30s test @ http://127.0.0.1:42069
  8 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     2.33ms    0.85ms  25.88ms   70.05%
    Req/Sec     5.18k   547.57    31.05k    99.38%
  1237550 requests in 30.10s, 165.23MB read
Requests/sec:  41115.64
Transfer/sec:      5.49MB
```

# Debug - No Logs

```
Running 30s test @ http://127.0.0.1:42069
  8 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     0.88ms  437.48us  40.06ms   90.48%
    Req/Sec    13.80k   632.79    31.23k    98.46%
  3299554 requests in 30.10s, 440.54MB read
Requests/sec: 109621.14
Transfer/sec:     14.64MB
```

# Release - Full Logs

```
Running 30s test @ http://127.0.0.1:42069
  8 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     4.38ms    1.51ms  16.36ms   68.65%
    Req/Sec     2.75k   249.47    10.14k    96.67%
  658506 requests in 30.10s, 87.92MB read
Requests/sec:  21877.94
Transfer/sec:      2.92MB
```

# Release - Less Logs

```
Running 30s test @ http://127.0.0.1:42069
  8 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.98ms  708.73us  12.91ms   68.84%
    Req/Sec     6.09k   544.73    23.86k    99.17%
  1455175 requests in 30.10s, 194.29MB read
Requests/sec:  48344.84
Transfer/sec:      6.45MB
```

# Release - No Logs

```
Running 30s test @ http://127.0.0.1:42069
  8 threads and 100 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency   339.23us  187.13us  12.00ms   78.21%
    Req/Sec    34.06k     2.31k  129.87k    95.50%
  8138513 requests in 30.10s, 1.06GB read
Requests/sec: 270384.49
Transfer/sec:     36.10MB
```
