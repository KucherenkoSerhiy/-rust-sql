use def;

pub fn create_database(db_name: String) -> String {
    "CREATE DATABASE IF NOT EXISTS ".to_string() + &db_name
}

pub fn use_database(db_name: String) -> String {
    "USE ".to_string() + &db_name
}

pub fn destroy_database(db_name: String) -> String {
    "DROP DATABASE IF EXISTS ".to_string() + &db_name
}

pub fn create_table(db_name: String, table: &def::DbTable) -> String{
    let mut load_table_query: String = "".to_string();
    load_table_query = load_table_query + "CREATE TABLE IF NOT EXISTS " + &db_name + "." + &table.name; load_table_query = load_table_query + "(
        " + &table.name + "_id INT NOT NULL PRIMARY KEY AUTO_INCREMENT"; for column in &table.columns {load_table_query = load_table_query + ",
        "+ &column.name + " "+ &column.db_type}; load_table_query = load_table_query +"
    );\n";
    load_table_query
}

pub fn perform_get(db_name: String, select_structure : &(&str, (&str, &str), Vec<&str>)) -> String{
    let last_column = select_structure.2.last().unwrap();
    let mut mysql_select: String = "SELECT ".to_string();
    for col in &select_structure.2{
        mysql_select = mysql_select + col;
        if col != last_column {mysql_select = mysql_select + ","};
        mysql_select = mysql_select + " "
    }
    mysql_select = mysql_select +

        "FROM " + &(db_name) + "." + select_structure.0 + " " +

        "WHERE " + (select_structure.1).0 + " = " + (select_structure.1).1 + ";";
    mysql_select
}

pub fn perform_post_mutation(db_name: String, insert_structure : &(&str, Vec<(&str, &str)> )) -> String{
    let last_column = &insert_structure.1.last().unwrap();

    let mut mysql_insert: String = "INSERT INTO ".to_string() + &db_name + "." + insert_structure.0 + "(";
    /*COLUMNS*/
    for col in &insert_structure.1{
        mysql_insert = mysql_insert + col.0;
        if col.0 != last_column.0 {mysql_insert = mysql_insert + ","};
        mysql_insert = mysql_insert + " ";
    }

    mysql_insert = mysql_insert + ")\n" +

        "VALUES (";
    for col in &insert_structure.1{
        mysql_insert = mysql_insert + "\"" + col.1 + "\"";;
        if col.1 != last_column.1 {mysql_insert = mysql_insert + ","};
        mysql_insert = mysql_insert + " ";
    }
    mysql_insert = mysql_insert + ");";
    mysql_insert
}

pub fn perform_update_mutation(db_name: String, update_structure : &(&str, (&str, &str), Vec<(&str, &str)> )) -> String{
    let last_column = &update_structure.2.last().unwrap();
    let mut mysql_update: String = "UPDATE ".to_string() + &db_name + "." + update_structure.0 + " SET ";
    /*COLUMNS*/
    for col in &update_structure.2{
        mysql_update = mysql_update + col.0 + " = " + col.1;
        if col.0 != last_column.0 {mysql_update = mysql_update + ","};
        mysql_update = mysql_update + " ";
    }

    mysql_update = mysql_update + "WHERE " + (update_structure.1).0 + " = " + (update_structure.1).1 + ";";

    mysql_update
}

pub fn perform_delete_mutation(db_name: String, delete_structure : &(&str, Option<(&str, &str)> )) -> String{
    let mut mysql_delete: String = "DELETE FROM ".to_string() + &db_name + "." + delete_structure.0 + " ";
    if let Some(id) = delete_structure.1 {
        mysql_delete = mysql_delete + "WHERE " + id.0 + "=" + id.1;
    }
    mysql_delete = mysql_delete + ";";
    mysql_delete
}