# SPDX-License-Identifier: AGPL-3.0-only
# Copyright (C) zer0.k, AlphaKeks <alphakeks@dawn.sh>

import numpy as np

from scipy import stats

def get_distribution_points_portion_under_50(time, wr_time, tier):
    return ((1+np.exp((2.1 - 0.25 * tier) * -0.5))/(1+np.exp((2.1 - 0.25 * tier) * (time/wr_time-1.5))))

def get_dist_points_portion(time, wr_time, dist: stats.rv_continuous, tier, top_scale, total):
    if total < 50:
        return get_distribution_points_portion_under_50(time, wr_time, tier)
    else:
        return np.clip(dist.sf(time) / top_scale, 0, 1)
