### 实现

exercise 6是要用channel实现一个`parallel_map`。思路大致是创建两个channel，一个channel用于发送input_vel中的数据给各个线程调用`U = f(T)`，同时另一个channel由主线程接收处理后的数据放入output_vel中。

为了确保output_vel的有序性，在channel中传递的每个数据都以元组标注了其对应的序列号：

```rust
let (t_sender, t_receiver): (Sender<(usize, T)>, Receiver<(usize, T)>) = crossbeam_channel::unbounded();
let (u_sender, u_receiver): (Sender<(usize, U)>, Receiver<(usize, U)>) = crossbeam_channel::unbounded();
```









### 其它注意事项

#### 对象上的生命周期

对象的存活时间只与生命周期最短的引用有关：

```rust
struct Solution<'b> {
    part: &'b str,
    name: String
}

fn main() {
    let mut s = Solution {part: "a", name: String::from("a")};
    {
        let b = String::from("123");
        s.part = &b;
    } // 在该括号后，s.part引用指向的内容被drop，导致整个s也失效.
    println!("{}", s.name);	// 此处会报错。
}
```



#### 关于'static

`'static`指明了当类型T为引用时，其生命周期必须与整个程序相同。在下述代码中，由于a的生命周期只在main中（而不是整个程序），故而会报错：

```rust
fn test<T: Send + Sync + std::fmt::Display + 'static>(val: T) {
    let handle = thread::spawn(move || println!("{}", val));
}

fn main() {
    let a = String::from("12345");
    test(&a);
}
```

报错如下：

```apl
error[E0597]: `a` does not live long enough
  --> src\main.rs:13:10
   |
13 |     test(&a);
   |     -----^^-
   |     |    |
   |     |    borrowed value does not live long enough
   |     argument requires that `a` is borrowed for `'static`
14 | }
   | - `a` dropped here while still borrowed
```

将main部分代码替换如下则可通过编译：

```rust
fn main() {
    let a = "123";
    test(a);
}
```

