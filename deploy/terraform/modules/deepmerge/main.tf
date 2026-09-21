variable "base" {
  description = "Generated values"
  type        = any
}

variable "override" {
  description = "Caller values laid over base: maps are merged key by key down to four levels, everything else (lists, scalars, deeper maps) replaces the generated value"
  type        = any
  default     = {}
}

locals {
  b = var.base
  o = var.override

  level4 = {
    for k1, v1 in local.b : k1 => {
      for k2, v2 in v1 : k2 => {
        for k3, v3 in v2 : k3 => {
          for k4, v4 in v3 : k4 => merge(v4, local.o[k1][k2][k3][k4])
          if can(keys(v4)) && can(keys(local.o[k1][k2][k3][k4]))
        }
        if can(keys(v3)) && can(keys(local.o[k1][k2][k3]))
      }
      if can(keys(v2)) && can(keys(local.o[k1][k2]))
    }
    if can(keys(v1)) && can(keys(local.o[k1]))
  }

  level3 = {
    for k1, v1 in local.b : k1 => {
      for k2, v2 in v1 : k2 => {
        for k3, v3 in v2 : k3 => merge(v3, local.o[k1][k2][k3], try(local.level4[k1][k2][k3], {}))
        if can(keys(v3)) && can(keys(local.o[k1][k2][k3]))
      }
      if can(keys(v2)) && can(keys(local.o[k1][k2]))
    }
    if can(keys(v1)) && can(keys(local.o[k1]))
  }

  level2 = {
    for k1, v1 in local.b : k1 => {
      for k2, v2 in v1 : k2 => merge(v2, local.o[k1][k2], try(local.level3[k1][k2], {}))
      if can(keys(v2)) && can(keys(local.o[k1][k2]))
    }
    if can(keys(v1)) && can(keys(local.o[k1]))
  }

  level1 = {
    for k1, v1 in local.b : k1 => merge(v1, local.o[k1], try(local.level2[k1], {}))
    if can(keys(v1)) && can(keys(local.o[k1]))
  }
}

output "result" {
  value     = merge(local.b, local.o, local.level1)
  sensitive = true
}
