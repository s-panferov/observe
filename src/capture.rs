pub trait Capture {
	type Output;

	fn capture(self) -> Self::Output;
}

impl<T> Capture for (&T,)
where
	T: Clone,
{
	type Output = (T,);
	fn capture(self) -> Self::Output {
		(self.0.clone(),)
	}
}

impl<T1, T2> Capture for (&T1, &T2)
where
	T1: Clone,
	T2: Clone,
{
	type Output = (T1, T2);
	fn capture(self) -> Self::Output {
		(self.0.clone(), self.1.clone())
	}
}

impl<T1, T2, T3> Capture for (&T1, &T2, &T3)
where
	T1: Clone,
	T2: Clone,
	T3: Clone,
{
	type Output = (T1, T2, T3);
	fn capture(self) -> Self::Output {
		(self.0.clone(), self.1.clone(), self.2.clone())
	}
}
