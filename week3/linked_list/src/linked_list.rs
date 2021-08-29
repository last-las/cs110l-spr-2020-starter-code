use std::fmt;
use std::option::Option;
use std::path::Iter;
use std::io::Take;

pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

#[derive(Debug)]
struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {value: value, next: next}
    }
}

impl<T: Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        if self.next.is_none() {
            Node::new(self.value.clone(), None)
        }else {
            Node::new(self.value.clone(), self.next.clone())
        }
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {head: None, size: 0}
    }
    
    pub fn get_size(&self) -> usize {
        self.size
    }
    
    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }
    
    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }
    
    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}


impl<T: fmt::Display> fmt::Display for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                },
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}

impl<T: Clone> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        LinkedList{ head: self.head.clone(), size: self.size.clone() }
    }
}

impl<T: PartialEq> PartialEq for LinkedList<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }else {
            let mut self_cur = &self.head;
            let mut other_cur = &other.head;
            while !self_cur.is_none() {
                if self_cur.as_ref().unwrap().value != other_cur.as_ref().unwrap().value {
                    return false;
                }

                self_cur = &self_cur.as_ref().unwrap().next;
                other_cur = & other_cur.as_ref().unwrap().next;
            }

            return true;
        }
    }
}

impl<T> Iterator for LinkedList<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop_front()
    }
}

pub struct LinkedListIter<'a, T> {
    current: &'a Option<Box<Node<T>>>,
}

impl<T> Iterator for LinkedListIter<'_, T> where T : Copy{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            Some(node) => {
                self.current = &node.next;
                Some(node.value)
            },
            None => None
        }
    }
}

impl<'a, T> IntoIterator for &'a LinkedList<T> where T :Copy {
    type Item = T;
    type IntoIter = LinkedListIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        LinkedListIter {current: &self.head}
    }
}

pub trait ComputeNorm<T> {
    fn compute_norm(&self) -> T;
}

impl ComputeNorm<f64> for LinkedList<f64>{
    fn compute_norm(&self) -> f64 {
        let mut sum:f64 = 0 as f64;
        for e in self {
            sum += e;
        }
        sum
    }
}