### 不要使用fork

1. 在使用fork时可能忘记调用`wait`导致子进程变为zombie浪费内存。

2. 当想要获得两个子进程时，可能导致程序出现未料想的执行结果：

   ```c
   int main() {
   	pid_t pid1 = fork();
   	pid_t pid2 = fork();	// there a actually two son processes and one grandson process
   	if (pid1 == 0)
   	{
   		// do stuff concurrently
   	}
   }
   ```

3. 当有人想要退出子进程（保证子进程不会执行父进程的内容）时，在main中可以使用return：

   ```cpp
   int main()
   {
   	pid_t pid1 = fork();
   	if(pid1 == 0)
   	{
   		cout << "hello" ;
   		return 0;
   	}
   	
   	// parent stuff
   }
   ```

   但当复制这段代码到函数中时，则子进程并不会退出：

   ```cpp
   int bad_function()
   {
   	pid_t pid1 = fork();
   	if(pid1 == 0)
   	{
   		cout << "hello" ;
   		return 0;
   	}
   }
   
   int main()
   {
   	bad_function();
   	// parent stuff
   }
   ```

4. 对于问题3正确的做法是使用execvp。

   但当使用execvp时，execvp错误时我们可能会`throw`，但是如果有人将抛出的错误catch了，然后子进程就又会去执行父进程的内容：

   ```cpp
   int bad_function()
   {
   	pid_t pid1 = fork();
   	if(pid1 == 0)
   	{
   		execvp(...);
   		throw "Could not find exe";
   	}
   }
   
   int main()
   {
   	try {
   		bad_function();
   	} catch(std:string e){
   		cout << "oh no!" << endl;
   	}
   	// parent stuff
   }
   ```

   

5. 如果在使用fork之前创建了多个线程，且子进程正在使用malloc时，主线程调用了fork，此时子进程会被摧毁，而此时堆上的内存会出现问题。当再次调用malloc时可能会失败：

   ```c
   // thread things
   pid_t pid = fork();
   if(pid == 0)
   {
   	char * a = malloc(sizeof(stuff));
   }
   ```



当想要并发执行两段代码时，在CS110D中的建议是：**在fork的进程中调用execvp去执行另外一段并发代码**。





### rust command

为了避免使用`fork`和`exec`带来的弊端，我们一般会定义一个更高层的抽象去处理这类需求。这类抽象在rust中如下，同时也运行定义一个"pre-exec function" 在fork和exec之间运行。



1. Build a Command：

   ```rust
   Command::new("ps")
   	.args(&["--pid", &pid.to_srtring()])
   ```

2. Run and get the output in a buffer：

   ```rust
   let output = Command::new("ps")
   	.args(&["--pid", &pid.to_srtring()])
   	.output()
   	.expect("Failed to execute")
   ```

   包含了status，stdout和stderr。

3. Run but only get the status code：

   ```rust
   let status = Command::new("ps")
   	.args(&["--pid", &pid.to_srtring()])
   	.status()
   	.expect("Failed to execute")
   ```

4. Spawn and immediately return：

   ```rust
   let child = Command::new("ps")
   	.args(&["--pid", &pid.to_srtring()])
   	.spawn()
   	.expect("Failed to execute")
   ```

   同时这里最后需要调用`wait`：

   ```rust
   let status = child.wait()
   ```

5. Pre-exec function

   ```rust
   let cmd = Command::new("ls");
   unsafe {
       cmd.pre_exec(function_to_run);
   }
   let child = cmd.spawn();
   ```

   





### 不要使用pipe

- 没有关闭文件描述符

- 错误的调用了`close`系统调用：

  ```rust
  if(close(fds[1] == -1)) {
  	printf("Error closing!");
  }
  ```

  正确的应该是：

  ```rust
  if(close(fds[1]) == -1) {
  	printf("Error closing!");
  }
  ```

- use before pipe

- use after close



### 不要使用signal

关于signal我所不知道的：

- 进程A给暂时阻塞的进程B发送n个信号，进程B被调度后只能看到一个信号。

  上面这句话说的学术些就是：系统不会对标准信号进行排队处理，也就是说，将信号标记为等待状态只会发生一次。

- 在执行某信号的处理器函数会阻塞同类信号的传递（除非在调用sigaction时指定了SA_NODEFER标志），如果在执行处理器函数时再次产生同类信号，那么会将该信号标记为等待状态并在处理器函数返回后再次传递（期间产生多次但只传第一次）

- 关于信号处理函数：
  - 确保信号处理函数代码本身是可重入的，且只调用异步信号安全的函数
  - 当主程序执行不安全函数或操作信号处理函数也可能更新全局数据结构时，阻塞信号的传递
  
- `wait`系统调用一般会返回被等待的子进程的pid，而当不再有能被等待的子进程时则返回-1。

- 在父进程执行`wait()`之前，其子进程就已经终止，则系统会释放子进程大部分资源，该进程唯一保留的是内核进程表中的一条记录，其中包含了子进程ID，终止状态，资源使用数据等信息。此时进程状态即为**僵尸进程**。

- 杀死**僵尸进程**的唯一方式就是kill掉父进程

- 对于子进程的处理，建议对**SIGCHLD**信号编写信号处理程序并调用`waitpid(-1, NULL, WNOHANG)`来`wait`掉所有已发生信号的子进程。



from CS110L：

- 不应该在signal handler函数中完成任何重要的事情（调用的函数很可能是“异步不安全的函数”）

- 这就意味着handler和主程序之间需要某种通信方式。一般的全局变量在这里并不好用，因为在handler执行期间能产生多次信号

- 解决方法是**self-pipe trick**

  1. create a pipe
  2. when awaiting a signal, read from the pipe(this will block until something is written to it)
  3. in the signal handler, write a single byte to the pipe 

  要保证主程序即能完成原本的任务，同时也实现**self-pipe trick**，有下述两种选择：

  - 线程
  - non-blocking I/O(week 8)





### process vs thread

两者相比：

- process切换上下文（虚拟内存等）耗费更多时间
- 每一个thread共享内存(虽然process也有写时复制)
- thread比process更好交换数据
- process**比**thread更安全，更隔离！