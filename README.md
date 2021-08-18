# Favilla: A set of Vulkan utilities

[![Documentation](https://docs.rs/favilla/badge.svg)](https://docs.rs/favilla/)
[![Crates.io](https://img.shields.io/crates/v/favilla.svg)](https://crates.io/crates/favilla)

## Overview

`favilla` provides some utilities for writing Vulkan code using [ash](https://github.com/MaikKlein/ash).

## Goals
The main goal of `favilla` is to provide commonly required functionality for Vulkan
while not getting in the way, giving the user full control and the option to use
as much or as little of `favilla`'s functionality as they want.

As an example, many structs in `favilla` offer construction methods that rely on some assumptions,
which can help reduce boilerplate code if those assumptions are met in your use case.
They also offer support for direct construction (through pub fields),
so it is possible to construct these objects manually and use their functionality,
even if the assumptions made by the construction methods don't hold.

## Licence
Licensed under
* MIT licence
* Apache Licence 2.0

## Contributions
Contributions are welcome! Unless explicitly stated otherwise, your contribution is
assumed to be licensed under the same licences as `favilla` (see above).
