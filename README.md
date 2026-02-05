# Ramb 𝞴-calculus interpreter

Progress tracking:
- [x] β-reduction
- [ ] α-equivalence
- [ ] η-equivalence
- [x] expression bindings
- [x] REPL
- [ ] File
- [ ] stdlib

```sh
➜ rlwrap cargo r -r --quiet
𝞴> succ :: \n f x. f (n f x)
𝞴n f x. f (n f x)
𝞴> 0 :: \f x. x
𝞴f x. x
𝞴> succ (succ 0)
(𝞴n f x. f (n f x)) ((𝞴n f x. f (n f x)) (𝞴f x. x))
    =ᵦ 𝞴f x. f ((𝞴n f x. f (n f x)) (𝞴f x. x) f x)
    =ᵦ 𝞴f x. f ((𝞴f x. f ((𝞴f x. x) f x)) f x)
    =ᵦ 𝞴f x. f ((𝞴f x. f ((𝞴x. x) x)) f x)
    =ᵦ 𝞴f x. f ((𝞴f x. f x) f x)
    =ᵦ 𝞴f x. f ((𝞴x. f x) x)
    =ᵦ 𝞴f x. f (f x)
𝞴> \wrong .. expression
error: Unexpected token, wanted Lambda found Dot at (1:8)
𝞴> 
```
