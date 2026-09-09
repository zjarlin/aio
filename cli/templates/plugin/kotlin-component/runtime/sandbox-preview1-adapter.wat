(module
  (import "__main_module__" "memory" (memory 0))
  (global $state (mut i32) (i32.const 1831565813))

  (func $next-byte (result i32)
    (local $value i32)
    global.get $state
    local.tee $value
    local.get $value
    i32.const 13
    i32.shl
    i32.xor
    local.tee $value
    local.get $value
    i32.const 17
    i32.shr_u
    i32.xor
    local.tee $value
    local.get $value
    i32.const 5
    i32.shl
    i32.xor
    local.tee $value
    global.set $state
    local.get $value
  )

  (func (export "random_get") (param $pointer i32) (param $length i32) (result i32)
    (local $index i32)
    (block $done
      (loop $fill
        local.get $index
        local.get $length
        i32.ge_u
        br_if $done
        local.get $pointer
        local.get $index
        i32.add
        call $next-byte
        i32.store8
        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $fill
      )
    )
    i32.const 0
  )
)
