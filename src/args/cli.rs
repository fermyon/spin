use std::ffi::OsString;

use clap::{Arg, Args, CommandFactory, FromArgMatches, Parser};

fn format_error<I: CommandFactory>(err: clap::Error) -> clap::Error {
    let mut cmd = I::command();
    err.format(&mut cmd)
}

pub struct App<P>
where
    P: Parser + CommandFactory,
{
    parser: P,
}

impl<P> App<P>
where
    P: Parser + CommandFactory,
{
    pub fn get_parser(self) -> P {
        self.parser
    }
}

impl<P> CommandFactory for App<P>
where
    P: Parser + CommandFactory,
{
    fn command() -> clap::Command {
        P::command()
    }

    fn command_for_update() -> clap::Command {
        P::command_for_update()
    }
}

impl<P> FromArgMatches for App<P>
where
    P: Parser + CommandFactory,
{
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        Ok(Self {
            parser: P::from_arg_matches(&matches)?,
        })
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        P::update_from_arg_matches(&mut self.parser, matches)
    }
}

impl<P> Parser for App<P>
where
    P: Parser + CommandFactory,
{
    fn parse() -> Self {
        let mut matches = <Self as clap::CommandFactory>::command().get_matches();
        let res = <Self as clap::FromArgMatches>::from_arg_matches_mut(&mut matches)
            .map_err(format_error::<Self>);
        match res {
            Ok(s) => s,
            Err(e) => {
                // Since this is more of a development-time error, we aren't doing as fancy of a quit
                // as `get_matches`
                e.exit()
            }
        }
    }

    fn try_parse() -> Result<Self, clap::Error> {
        let mut matches = <P as clap::CommandFactory>::command().try_get_matches()?;
        Ok(Self {
            parser: <P as clap::FromArgMatches>::from_arg_matches_mut(&mut matches)
                .map_err(format_error::<P>)?,
        })
    }

    fn parse_from<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut matches = <P as clap::CommandFactory>::command().get_matches_from(itr);
        let res = <P as clap::FromArgMatches>::from_arg_matches_mut(&mut matches)
            .map_err(format_error::<P>);
        match res {
            Ok(s) => Self { parser: s },
            Err(e) => {
                // Since this is more of a development-time error, we aren't doing as fancy of a quit
                // as `get_matches_from`
                e.exit()
            }
        }
    }

    fn try_parse_from<I, T>(itr: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut matches = <P as clap::CommandFactory>::command().try_get_matches_from(itr)?;
        Ok(Self {
            parser: <P as clap::FromArgMatches>::from_arg_matches_mut(&mut matches)
                .map_err(format_error::<P>)?,
        })
    }

    fn update_from<I, T>(&mut self, itr: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let mut matches = <P as clap::CommandFactory>::command_for_update().get_matches_from(itr);
        let res = <P as clap::FromArgMatches>::update_from_arg_matches_mut(
            &mut self.parser,
            &mut matches,
        )
        .map_err(format_error::<P>);
        if let Err(e) = res {
            // Since this is more of a development-time error, we aren't doing as fancy of a quit
            // as `get_matches_from`
            e.exit()
        }
    }
}

impl<P> App<P>
where
    P: Parser + CommandFactory,
{
    pub fn without_errors() -> Result<Self, clap::Error> {
        Ok(Self {
            parser: P::from_arg_matches(&P::command().ignore_errors(true).try_get_matches()?)?,
        })
    }
    pub fn start<O>(map_err: O) -> Result<Self, clap::Error>
    where
        O: FnOnce(Result<Self, clap::Error>, Self) -> Result<Self, clap::Error>,
    {
        map_err(Self::try_parse(), Self::without_errors()?)
    }
}

#[derive(Debug)]
pub struct Command<P>
where
    P: Parser,
{
    args: P,
}

impl<P> Command<P>
where
    P: Parser,
{
    pub fn get_args(self) -> P {
        self.args
    }

    pub fn catch(self, error: clap::Error) -> ! {
        error.exit()
    }
}

impl<P> FromArgMatches for Command<P>
where
    P: Parser,
{
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        Ok(Self {
            args: P::from_arg_matches(&matches)?,
        })
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        P::update_from_arg_matches(&mut self.args, matches)
    }
}

impl<P> Args for Command<P>
where
    P: Args + Parser,
{
    fn augment_args(cmd: clap::Command) -> clap::Command {
        P::augment_args(cmd)
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        P::augment_args_for_update(cmd)
    }
}

pub fn mut_subs<F>(f: F) -> impl Fn(clap::Command) -> clap::Command
where
    F: Fn(clap::Command) -> clap::Command,
{
    move |cmd| {
        cmd.clone()
            .get_subcommands_mut()
            .fold(cmd, |cmd, sub| cmd.mut_subcommand(sub.get_name(), &f))
    }
}

pub fn mut_args<F>(f: F) -> impl Fn(clap::Command) -> clap::Command
where
    F: Fn(Arg) -> Arg,
{
    move |cmd| {
        cmd.clone()
            .get_arguments()
            .fold(cmd, |cmd, arg| cmd.mut_arg(arg.get_id(), &f))
    }
}
