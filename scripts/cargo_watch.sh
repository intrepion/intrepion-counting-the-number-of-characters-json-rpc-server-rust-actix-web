#!/usr/bin/env bash

cargo watch -x fmt -x clippy -x check -x test -x audit -x "+nightly udeps"
