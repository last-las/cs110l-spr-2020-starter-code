Copy trait：编译器自动使用，比如在使用`=`号时

Clone trait：手动调用



rust的泛型可以用在函数上：

```rust
fn identity_fn<T>(x: T) -> T {x}
```



unique pointer：只有一个对象可以拥有一个指向堆的指针



`Rc<T>`可能会导致内存泄露（`p1->p2->p1`）



在方法定义中使用泛型必须如下：

```rust
impl<T> Point<T> {

}
```

注意impl后必须添加`<T>`，因为要和下面的情况区分：

```rust
impl Point<u32> {

}
```

