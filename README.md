# Favilla: A set of Vulkan utilities

[![Documentation](https://docs.rs/favilla/badge.svg)](https://docs.rs/favilla/)
[![Crates.io](https://img.shields.io/crates/v/favilla.svg)](https://crates.io/crates/favilla)

## Overview

`favilla` provides some utilities for writing Vulkan code using [ash](https://github.com/MaikKlein/ash).

## Goals
The main goal of `favilla` is to provide commonly required functionality for Vulkan
while not getting in the way, giving the user full control and the option to use
as much or as little of `favilla`'s functionality as they want.

As an example, many structs in `favilla` offer construction methods that make certain assumptions,
which can help reduce boilerplate code.
There is also the option to directly construction these structs (through pub fields),
so it is possible to use the utility methods offered by these types,
even if the assumptions made by the construction methods don't hold.    

`favilla` tries to make getting started easy and targets the most common use cases in 
a relatively easy-to-use way.  
It does not try to be an all-encompassing abstraction of Vulkan. 
Sometimes, there is no way to work around the assumptions made by `favilla`;
in such cases, your application should use `ash` directly to synchronize resources according to your needs.  
For example, when uploading data from a staging buffer to an image
`favilla` assumes that your images will be used by a fragment shader, not by a vertex shader, and 
sets the stages and masks in the memory barriers used for synchronization accordingly.  
The source code of `favilla` should be helpful for making the necessary adjustments in your application directly.

## Licence
Licensed under
* MIT licence
* Apache Licence 2.0

## Contributions
Unless explicitly stated otherwise, all contributions are 
assumed to be licensed under the same licences as `favilla` (see above).
