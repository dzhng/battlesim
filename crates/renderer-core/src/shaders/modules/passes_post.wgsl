// WGSL owner: passes_post.

// @block passes_post_rawFragment
@fragment fn rawFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return rawShader(input.uv);
}

// @block passes_post_bloomCompositeFragment
@fragment fn bloomCompositeFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return compositeShader(input.uv);
}

// @block passes_post_directFragment
@fragment fn directFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return directShader(input.uv);
}

// @block passes_post_finalFragment
@fragment fn finalFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return finalShader(input.uv);
}

// @block passes_post_highpassFragment
@fragment fn highpassFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return highpass(input.uv);
}

// @block passes_post_highPlaneFragment
@fragment fn highPlaneFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return highPlaneShader(input.uv);
}

// @block passes_post_blurFragment
@fragment fn blurFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return blurShader(input.uv);
}

// @block passes_post_hdrFragment
@fragment fn hdrFragment(input: ScreenFragmentInput) -> @location(0) vec4f {
  return hdrShader(input.uv);
}
