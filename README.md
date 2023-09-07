# Structur

### A small library for making the definition of multiple similar structures quick

## Example

In this example, we create structs representing a user and the actions you may take on them,
code like this is very common in API design

```rust 

struct User{
    pub id : u64,
    pub first_name : String,
    pub second_name : String,
    pub age : u64,
}

struct CreateUser{
    pub first_name : String,
    pub second_name : String;
    pub age : u64,
    pub password : String
}

struct UpdateUser{
    pub id : u64,
    pub first_name : Option<String>,
    pub second_name : Option<String>,
    pub age : Option<u64>,
    pub password : Option<String>,
}

```

with structur, you have to write the definitions once, and when a field is changed,
that change is automatically reflected in all derived structures, which leads to less errors

```rust

#[structur::structur(create = CreateUser, update = UpdateUser, show = User)]
struct User{

    #[hide(create)]
    pub id : u64,

    #[optional(update)]
    pub first_name : String,

    #[optional(update)]
    pub second_name : String,

    #[optional(update)]
    pub age : u64,

    #[hide(show)]
    pub password : String
}
```

We now have the structs CreateUser, UpdateUser and User defined from the single initial definition struct
