# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) zer0.k, AlphaKeks <alphakeks@dawn.sh>

import common
import json
import scipy.stats as stats
import sys
import sys
import traceback

def process_input(line):
    """
    Processes a single line read from stdin.

    The line is expected to contain a JSON object with the following keys:
        * `time` - the time of the run, in seconds, as a floating point number
        * `nub_data` - an object containing information about the nub
                       leaderboard the run belongs to
        * `pro_data` - an object containing information about the pro
                       leaderboard the run belongs to

    Both `nub_data` and `pro_data` should contain the following keys:
        * `tier` - the filter tier
        * `wr` - the time of the world record run, in seconds, as a floating
                 point number
        * `leaderboard_size` - the number of runs on the leaderboard
        * `dist_params` - distribution parameters calculated by `calc_filter.py`

    An example object could look like this:
        ```json
        {
          "time": 8.609375,
          "nub_data": {
            "tier": 1,
            "wr": 7.6484375,
            "leaderboard_size": 224,
            "dist_params": {
              "a": 33.53900289787477,
              "b": 33.52140111667502,
              "loc": 6.3663207368487065,
              "scale": 0.4480388195262859,
              "top_scale": 0.9979285278452101
            }
          },
          "pro_data": {
            "tier": 1,
            "wr": 7.6484375,
            "leaderboard_size": 165,
            "dist_params": {
              "a": 2.6294814553333743,
              "b": 2.511121972118702,
              "loc": 8.713014153227697,
              "scale": 2.2226724397990805,
              "top_scale": 0.9952929135343108
            }
          }
        }
        ```

    The function will write a single line response to stdout.

    That line is a JSON object with the following keys:
        * `nub_fraction` - a floating point number
        * `pro_fraction` - a floating point number

    An example object could look like this:
        ```json
        {
          "nub_fraction": 0.9745534941686896,
          "pro_fraction": 0.9760910013054752
        }
        ```
    """

    data = json.loads(line)

    nub_data = data['nub_data']
    nub_dist = stats.norminvgauss(
        a = nub_data['dist_params']['a'],
        b = nub_data['dist_params']['b'],
        loc = nub_data['dist_params']['loc'],
        scale = nub_data['dist_params']['scale']
    )
    nub_fraction = common.get_dist_points_portion(data['time'],
        nub_data['wr'],
        nub_dist,
        nub_data['tier'],
        nub_data['dist_params']['top_scale'],
        nub_data['leaderboard_size'])
    if 'pro_data' in data and data['pro_data'] is not None:
        pro_data = data['pro_data']
        pro_dist = stats.norminvgauss(
            a = pro_data['dist_params']['a'],
            b = pro_data['dist_params']['b'],
            loc = pro_data['dist_params']['loc'],
            scale = pro_data['dist_params']['scale']
        )
        pro_fraction = common.get_dist_points_portion(data['time'],
            pro_data['wr'],
            pro_dist,
            pro_data['tier'],
            pro_data['dist_params']['top_scale'],
            pro_data['leaderboard_size'])
        response = {
            'nub_fraction': nub_fraction,
            # Pro run in the pro leaderboard should never be worth less than
            # the same run in the nub leaderboard.
            'pro_fraction': max(nub_fraction, pro_fraction)
        }
        return response
    response = {
        'nub_fraction': nub_fraction,
        'pro_fraction': None
    }
    return response

def main():
    for line in sys.stdin:
        try:
            response = process_input(line)
            sys.stderr.flush()
            sys.stdout.write(json.dumps(response) + '\n')
            sys.stdout.flush()
        except KeyError as e:
            sys.stderr.write(f'Missing key in input data: {e}\n')
            sys.stderr.write(traceback.format_exc() + '\n')
            sys.exit(1)
        except json.JSONDecodeError as e:
            sys.stderr.write(f'JSON decode error: {e}\n')
            sys.stderr.write(traceback.format_exc() + '\n')
            sys.exit(1)
        except Exception as e:
            sys.stderr.write(f'An unexpected error occurred: {e}\n')
            sys.stderr.write(traceback.format_exc() + '\n')
            sys.exit(1)

if __name__ == '__main__':
    main()
