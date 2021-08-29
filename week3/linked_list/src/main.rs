use linked_list::LinkedList;
pub mod linked_list;

fn main() {

}

#[cfg(test)]
mod test{
    use crate::linked_list::{LinkedList, ComputeNorm};

    #[test]
    fn test_into_iterator() {
        let mut  list = LinkedList::new();

        for i in 1..12 {
            list.push_front(i);
        }

        let mut s = String::new();
        for e in list {
            s = format!("{} {}", e, s);
        }
        println!("{}", s);
    }

    #[test]
    fn test_into_iterator_ampersand() {
        let mut  list = LinkedList::new();

        for i in 1..12 {
            list.push_front(i);
        }

        let mut s = String::new();
        for e in &list {
            s = format!("{} {}", s, e);
        }
        assert_eq!(s.trim(), format!("{}", list).trim());
    }

    #[test]
    fn test_compute_norm() {
        let mut list:LinkedList<f64> = LinkedList::new();

        for i in 1..12{
            list.push_front(i as f64);
        }

        assert_eq!(list.compute_norm(), 66 as f64);
    }
}
