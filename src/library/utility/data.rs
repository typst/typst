use crate::library::prelude::*;

/// Read structured data from a CSV file.
pub fn csv(vm: &mut Vm, args: &mut Args) -> TypResult<Value> {
    let Spanned { v: path, span } =
        args.expect::<Spanned<EcoString>>("path to csv file")?;

    let path = vm.locate(&path).at(span)?;
    let try_load = || -> io::Result<Value> {
        let data = vm.world.file(&path)?;

        let mut builder = csv::ReaderBuilder::new();
        builder.has_headers(false);

        let mut reader = builder.from_reader(data.as_slice());
        let mut vec = vec![];

        for result in reader.records() {
            vec.push(Value::Array(
                result?.iter().map(|field| Value::Str(field.into())).collect(),
            ))
        }

        Ok(Value::Array(Array::from_vec(vec)))
    };

    try_load()
        .map_err(|err| failed_to_load("csv file", &path, err))
        .at(span)
}
